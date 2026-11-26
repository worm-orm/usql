//! Needs a disposable database, and does nothing without one:
//! `USQL_MYSQL_URL=mysql://root:pw@localhost:3306/usql cargo test -p usql-mysql`

use futures::{StreamExt, TryStreamExt};
use usql::{
    FromRow,
    builder::{
        StatementExt,
        expr::val,
        mutate::{Set, insert},
        schema::{Column, ColumnType, create_table},
        select::{FilterQuery, QueryExt, select},
    },
    core::{
        ColumnIndex, Connection, Connector, DatabaseInfo, Executor, Pool, Row, System, Transaction,
    },
    value::{JsonValue, Type, Value, chrono::NaiveDate},
};
use usql_mysql::{Mysql, MysqlConn, MysqlError, MysqlOptions, MysqlRow};

macro_rules! url {
    () => {
        match std::env::var("USQL_MYSQL_URL") {
            Ok(url) => url,
            Err(_) => {
                eprintln!("skipped: USQL_MYSQL_URL is not set");
                return;
            }
        }
    };
}

async fn connect(url: &str) -> MysqlConn {
    let pool = Mysql::create_pool(MysqlOptions::from_url(url))
        .await
        .unwrap();
    pool.get().await.unwrap()
}

async fn query<E: Executor<Connector = Mysql>>(
    conn: &E,
    sql: &str,
    params: Vec<Value>,
) -> Result<Vec<MysqlRow>, MysqlError> {
    let mut stmt = conn.prepare(sql).await?;
    let params = params.into_iter().map(Into::into).collect();
    conn.query(&mut stmt, params).try_collect().await
}

async fn exec<E: Executor<Connector = Mysql>>(
    conn: &E,
    sql: &str,
    params: Vec<Value>,
) -> Result<(), MysqlError> {
    let mut stmt = conn.prepare(sql).await?;
    let params = params.into_iter().map(Into::into).collect();
    conn.exec(&mut stmt, params).await
}

async fn one<E: Executor<Connector = Mysql>>(
    conn: &E,
    sql: &str,
    params: Vec<Value>,
) -> Result<Value, MysqlError> {
    let rows = query(conn, sql, params).await?;
    let value = rows[0].get(ColumnIndex::Index(0))?.to_owned();
    Ok(value)
}

async fn count<E: Executor<Connector = Mysql>>(conn: &E, table: &str) -> Value {
    one(conn, &format!("SELECT count(*) FROM {table}"), vec![])
        .await
        .unwrap()
}

#[tokio::test]
async fn values() {
    let conn = connect(&url!()).await;
    assert_eq!(conn.db_info().variant(), System::Mysql);

    let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    let time = date.and_hms_micro_opt(13, 14, 15, 123456).unwrap();
    let json = JsonValue::Object(
        [(
            "a".to_string(),
            JsonValue::Array(vec![JsonValue::Null, JsonValue::String("\"q\"".into())]),
        )]
        .into_iter()
        .collect(),
    );
    let uuid = uuid::Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();

    // Columns made by the builder, and what is read back without a type
    let columns = [
        (ColumnType::Bool, Value::Bool(true), None),
        (ColumnType::SmallInt, Value::SmallInt(-5), None),
        (ColumnType::Int, Value::Int(i32::MIN), None),
        (ColumnType::BigInt, Value::BigInt(i64::MAX), None),
        (ColumnType::Float, Value::from(1.5f32), None),
        (ColumnType::Double, Value::from(-0.1f64), None),
        (ColumnType::Text, Value::from("blåbær"), None),
        (ColumnType::VarChar(10), Value::from(""), None),
        (
            ColumnType::Blob,
            Value::ByteArray(vec![0, 255].into()),
            None,
        ),
        (ColumnType::Date, Value::Date(date), None),
        (ColumnType::Time, Value::Time(time.time()), None),
        (ColumnType::DateTime, Value::Timestamp(time), None),
        (
            ColumnType::Uuid,
            Value::Uuid(uuid),
            Some(Value::ByteArray(uuid.as_bytes().to_vec().into())),
        ),
        (ColumnType::Json, Value::Json(json), None),
        (
            ColumnType::Other("DECIMAL(30,7)".into()),
            Value::from("-12345678901234567890.0123400"),
            None,
        ),
        (
            ColumnType::Other("BIGINT UNSIGNED".into()),
            Value::from(u64::MAX.to_string()),
            None,
        ),
        (
            ColumnType::Other("TINYINT".into()),
            Value::SmallInt(-128),
            None,
        ),
    ];

    let mut table = create_table("usql_values");
    for (i, (ty, _, _)) in columns.iter().enumerate() {
        table = table.column(Column::new(format!("c{i}"), ty.clone()));
    }
    conn.exec_batch("DROP TABLE IF EXISTS usql_values")
        .await
        .unwrap();
    let sql = table.to_sql(System::Mysql).unwrap();
    exec(&conn, &sql.sql, vec![]).await.unwrap();

    let names = (0..columns.len())
        .map(|i| format!("c{i}"))
        .collect::<Vec<_>>();
    let marks = vec!["?"; columns.len()].join(",");
    let insert = format!(
        "INSERT INTO usql_values ({}) VALUES ({marks})",
        names.join(",")
    );
    // A row with values, and one with nulls
    exec(
        &conn,
        &insert,
        columns.iter().map(|(_, v, _)| v.clone()).collect(),
    )
    .await
    .unwrap();
    exec(&conn, &insert, vec![Value::Null; columns.len()])
        .await
        .unwrap();

    let rows = query(&conn, "SELECT * FROM usql_values", vec![])
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    for (i, (ty, value, untyped)) in columns.iter().enumerate() {
        let read = rows[0].get(ColumnIndex::Index(i)).unwrap().to_owned();
        assert_eq!(&read, untyped.as_ref().unwrap_or(value), "{ty:?}");
        let null = rows[1].get(ColumnIndex::Index(i)).unwrap().to_owned();
        assert_eq!(null, Value::Null, "{ty:?}");
    }
    let typed = rows[0]
        .get_typed(ColumnIndex::Named("c12"), Type::Uuid)
        .unwrap()
        .to_owned();
    assert_eq!(typed, Value::Uuid(uuid));

    // The server agrees about what the decimal is
    let same = one(
        &conn,
        "SELECT c14 = CAST(? AS DECIMAL(30,7)) FROM usql_values LIMIT 1",
        vec!["-12345678901234567890.01234".into()],
    )
    .await
    .unwrap();
    assert_eq!(same, Value::BigInt(1));

    // Durations don't fit a time of day
    assert_eq!(
        one(&conn, "SELECT CAST('-30:00:01' AS TIME)", vec![])
            .await
            .unwrap(),
        Value::from("-30:00:01.000000")
    );
    assert!(
        exec(&conn, "SELECT ?", vec![Value::Array(vec![])])
            .await
            .is_err()
    );

    conn.exec_batch("DROP TABLE usql_values").await.unwrap();
}

#[tokio::test]
async fn typed() {
    let conn = connect(&url!()).await;
    let rows = query(
        &conn,
        "SELECT 1 AS a, CAST(2.50 AS DECIMAL(5,2)) AS b, '{\"x\":1}' AS c, NULL AS d,
                40000 AS e, (SELECT SUM(n) FROM (SELECT 1 AS n UNION ALL SELECT 2) AS t) AS f",
        vec![],
    )
    .await
    .unwrap();
    let row = &rows[0];
    let get = |name, ty| {
        row.get_typed(ColumnIndex::Named(name), ty)
            .map(|m| m.to_owned())
    };
    assert_eq!(get("a", Type::Int).unwrap(), Value::Int(1));
    assert_eq!(get("b", Type::Double).unwrap(), Value::from(2.5f64));
    assert!(get("b", Type::Int).is_err());
    assert!(matches!(get("c", Type::Json).unwrap(), Value::Json(_)));
    assert_eq!(get("d", Type::Int).unwrap(), Value::Null);
    assert!(matches!(
        get("e", Type::SmallInt),
        Err(MysqlError::Convert { .. })
    ));
    // SUM gives a decimal
    assert_eq!(get("f", Type::BigInt).unwrap(), Value::BigInt(3));
    assert!(matches!(get("?", Type::Int), Err(MysqlError::NotFound)));
    assert!(matches!(
        row.get(ColumnIndex::Index(6)),
        Err(MysqlError::NotFound)
    ));
    assert_eq!(row.len(), 6);
    assert_eq!(row.column_name(1), Some("b"));
    assert_eq!(row.column_name(6), None);
}

#[tokio::test]
async fn errors() {
    let conn = connect(&url!()).await;
    let err = query(&conn, "SELECT * FROM usql_missing", vec![])
        .await
        .err()
        .unwrap();
    assert!(err.to_string().contains("usql_missing"), "{err}");

    // Wrong number of parameters
    for params in [vec![], vec![Value::Int(1), Value::Int(2)]] {
        assert!(query(&conn, "SELECT ?", params).await.is_err());
    }
    // The connection is still usable
    assert_eq!(
        one(&conn, "SELECT ?", vec![1i64.into()]).await.unwrap(),
        Value::BigInt(1)
    );

    assert!(
        Mysql::create_pool(MysqlOptions::from_url("not a url"))
            .await
            .is_err()
    );
    let pool = Mysql::create_pool(MysqlOptions::from_url("mysql://localhost:1/db"))
        .await
        .unwrap();
    assert!(pool.get().await.is_err());
}

#[tokio::test]
async fn unfinished_stream() {
    let conn = connect(&url!()).await;
    let mut stmt = conn
        .prepare("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3")
        .await
        .unwrap();
    {
        let mut stream = conn.query(&mut stmt, vec![]);
        let first = stream.next().await.unwrap().unwrap();
        assert_eq!(
            first.get(ColumnIndex::Index(0)).unwrap().to_owned(),
            Value::BigInt(1)
        );
    }
    // The rest of the result is dropped
    assert_eq!(
        one(&conn, "SELECT 4", vec![]).await.unwrap(),
        Value::BigInt(4)
    );
}

#[tokio::test]
async fn use_database() {
    let conn = connect(&url!()).await;
    let current = "SELECT DATABASE()";
    let start = one(&conn, current, vec![]).await.unwrap();
    conn.exec_batch("CREATE DATABASE IF NOT EXISTS usql_use; USE usql_use")
        .await
        .unwrap();
    // Not the statement prepared before
    assert_eq!(
        one(&conn, current, vec![]).await.unwrap(),
        Value::from("usql_use")
    );
    let Value::Text(start) = start else {
        panic!("{start:?}")
    };
    conn.exec_batch(&format!("USE `{start}`; DROP DATABASE usql_use"))
        .await
        .unwrap();
    assert_eq!(
        one(&conn, current, vec![]).await.unwrap(),
        Value::from(start.as_str())
    );
}

#[tokio::test]
async fn transactions() {
    let mut conn = connect(&url!()).await;
    conn.exec_batch("CREATE TEMPORARY TABLE tx(id INT) ENGINE=InnoDB")
        .await
        .unwrap();

    for (id, commit) in [(1, Some(true)), (2, Some(false)), (3, None)] {
        let tx = conn.begin().await.unwrap();
        exec(&tx, "INSERT INTO tx VALUES (?)", vec![Value::Int(id)])
            .await
            .unwrap();
        assert_eq!(
            count(&tx, "tx").await,
            Value::BigInt(if id == 1 { 1 } else { 2 })
        );
        match commit {
            Some(true) => tx.commit().await.unwrap(),
            Some(false) => tx.rollback().await.unwrap(),
            None => drop(tx),
        }
        assert_eq!(count(&conn, "tx").await, Value::BigInt(1), "{id}");
    }

    // Several statements at once
    let tx = conn.begin().await.unwrap();
    tx.exec_batch("INSERT INTO tx VALUES (4); INSERT INTO tx VALUES (5);")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(count(&conn, "tx").await, Value::BigInt(3));
}

#[tokio::test]
async fn abandoned_transaction_returned_to_pool() {
    let options = MysqlOptions::from_url(url!()).max_size(1);
    let pool = Mysql::create_pool(options).await.unwrap();
    pool.get()
        .await
        .unwrap()
        .exec_batch("CREATE TABLE IF NOT EXISTS usql_abandoned(id INT) ENGINE=InnoDB; DELETE FROM usql_abandoned")
        .await
        .unwrap();

    {
        let mut conn = pool.get().await.unwrap();
        let tx = conn.begin().await.unwrap();
        exec(&tx, "INSERT INTO usql_abandoned VALUES (1)", vec![])
            .await
            .unwrap();
        drop(tx);
    }

    let conn = pool.get().await.unwrap();
    assert_eq!(count(&conn, "usql_abandoned").await, Value::BigInt(0));
    conn.exec_batch("DROP TABLE usql_abandoned").await.unwrap();
}

#[tokio::test]
async fn pool() {
    let options = MysqlOptions::from_url(url!()).max_size(1);
    let pool = Mysql::create_pool(options).await.unwrap();
    let mut ids = Vec::new();
    for pool in [pool.clone(), pool] {
        let conn = pool.get().await.unwrap();
        ids.push(one(&conn, "SELECT CONNECTION_ID()", vec![]).await.unwrap());
    }
    assert_eq!(ids[0], ids[1]);
}

#[derive(Debug, FromRow)]
struct User {
    id: i32,
    name: String,
    age: i64,
}

#[tokio::test]
async fn facade() {
    // Not at the top, as it adds methods to strings
    use usql::builder::expr::ExpressionExt;

    let pool = usql::Pool::<Mysql>::open(MysqlOptions::from_url(url!()))
        .await
        .unwrap();
    let conn = pool.conn().await.unwrap();

    conn.exec("DROP TABLE IF EXISTS usql_user").await.unwrap();
    conn.exec(
        create_table("usql_user")
            .column(Column::new("id", ColumnType::Int).auto(true).primary_key())
            .column(Column::new("name", ColumnType::Text).required(true))
            .column(Column::new("age", ColumnType::BigInt)),
    )
    .await
    .unwrap();

    for (name, age) in [("Rasmus", Value::Int(40)), ("Wilbur", Value::Null)] {
        conn.exec(insert("usql_user").with("name", val(name)).with("age", age))
            .await
            .unwrap();
    }

    let user = conn
        .fetch_one(
            select("usql_user", ("id", "name", "age"))
                .filter("id".eql(val(1)))
                .into_stmt(),
        )
        .await
        .unwrap();
    let user = User::from_row(user).unwrap();
    assert_eq!((user.id, user.name.as_str(), user.age), (1, "Rasmus", 40));

    let row = conn
        .fetch_one(
            select("usql_user", ("id", "name", "age"))
                .filter("id".eql(val(2)).and("name".eql(val("Wilbur"))))
                .into_stmt(),
        )
        .await
        .unwrap();
    assert_eq!(row.try_get::<i32, _>("id").unwrap(), 2);
    assert_eq!(row.get("age").unwrap().to_owned(), Value::Null);

    // An error, and not a panic
    assert!(
        conn.fetch_one(select("usql_missing", ("id",)).into_stmt())
            .await
            .is_err()
    );

    conn.exec("DROP TABLE usql_user").await.unwrap();
}

#[tokio::test]
async fn any() {
    let config = usql_any::Config {
        workers: None,
        kind: usql_any::DatabaseConfig::Mysql(usql_any::MysqlConfig {
            url: url!(),
            max_connections: Some(1),
        }),
    };
    let pool = config.crate_pool().await.unwrap();
    let mut conn = pool.get().await.unwrap();
    assert_eq!(conn.db_info().variant(), System::Mysql);

    let tx = conn.begin().await.unwrap();
    tx.exec_batch("CREATE TEMPORARY TABLE any_test(id INT)")
        .await
        .unwrap();
    let mut stmt = tx.prepare("INSERT INTO any_test VALUES (?)").await.unwrap();
    tx.exec(&mut stmt, vec![Value::Int(7).into()])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut stmt = conn.prepare("SELECT id FROM any_test").await.unwrap();
    let rows: Vec<_> = conn.query(&mut stmt, vec![]).try_collect().await.unwrap();
    assert_eq!(
        rows[0].get(ColumnIndex::Named("id")).unwrap().to_owned(),
        Value::Int(7)
    );
    assert!(conn.prepare("SELECT * FROM usql_missing").await.is_err());
}

#[tokio::test]
async fn migrate() {
    let pool = Mysql::create_pool(MysqlOptions::from_url(url!()))
        .await
        .unwrap();
    let reset = "DROP TABLE IF EXISTS usql_migrations, usql_migrated";
    pool.get().await.unwrap().exec_batch(reset).await.unwrap();

    let dir = std::env::temp_dir().join(format!("usql-mysql-{}", std::process::id()));
    for (name, sql) in [
        ("1_first", "CREATE TABLE usql_migrated(id INT)"),
        (
            "2_second",
            "INSERT INTO usql_migrated VALUES (1); INSERT INTO usql_migrated VALUES (2);",
        ),
    ] {
        std::fs::create_dir_all(dir.join(name)).unwrap();
        std::fs::write(dir.join(name).join("up.mysql"), sql).unwrap();
    }

    let migrator = usql_migrate::Migrator::<Mysql, _>::new(
        pool.clone(),
        usql_migrate::sql::SqlLoader,
        dir.clone(),
        "usql_migrations".to_string(),
    );
    assert!(migrator.has_migrations().await.unwrap());
    assert!(migrator.migrate_all().await.unwrap());
    assert!(!migrator.migrate_all().await.unwrap());
    let migrations = migrator.list_migrations().await.unwrap();
    assert_eq!(migrations.len(), 2);
    assert!(migrations.iter().all(|m| m.applied.is_some()));

    let conn = pool.get().await.unwrap();
    assert_eq!(count(&conn, "usql_migrated").await, Value::BigInt(2));

    conn.exec_batch(reset).await.unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
