//! Needs a database with PostGIS, and does nothing without one:
//! `USQL_POSTGRES_URL=postgres://postgres:postgres@localhost/postgres cargo test -p usql-postgres`
//!
//! The connection with TLS is only tested with the `rustls` feature, and when
//! `USQL_POSTGRES_CA` is the path of a certificate (PEM) to trust.

use futures::TryStreamExt;
use usql::{
    FromRow,
    builder::{
        expr::val,
        mutate::{Set, insert},
        schema::{Column, ColumnType, create_table},
        select::{FilterQuery, QueryExt, select},
    },
    core::{
        ColumnIndex, Connection, Connector, DatabaseInfo, Executor, Pool, Row, System, Transaction,
    },
    value::{JsonValue, Type, Value, chrono::NaiveDate, geob::Geob},
};
use usql_postgres::{Postgres, PostgresConn, PostgresError, PostgresOptions, PostgresRow};

macro_rules! url {
    () => {
        match std::env::var("USQL_POSTGRES_URL") {
            Ok(url) => url,
            Err(_) => {
                eprintln!("skipped: USQL_POSTGRES_URL is not set");
                return;
            }
        }
    };
}

async fn connect(url: &str) -> PostgresConn {
    let pool = Postgres::create_pool(PostgresOptions::from_url(url))
        .await
        .unwrap();
    pool.get().await.unwrap()
}

async fn query<E: Executor<Connector = Postgres>>(
    conn: &E,
    sql: &str,
    params: Vec<Value>,
) -> Result<Vec<PostgresRow>, PostgresError> {
    let mut stmt = conn.prepare(sql).await?;
    let params = params.into_iter().map(Into::into).collect();
    conn.query(&mut stmt, params).try_collect().await
}

async fn one<E: Executor<Connector = Postgres>>(
    conn: &E,
    sql: &str,
    params: Vec<Value>,
) -> Result<Value, PostgresError> {
    let rows = query(conn, sql, params).await?;
    let value = rows[0].get(ColumnIndex::Index(0))?.to_owned();
    Ok(value)
}

async fn count<E: Executor<Connector = Postgres>>(conn: &E, table: &str) -> Value {
    one(conn, &format!("SELECT count(*) FROM {table}"), vec![])
        .await
        .unwrap()
}

#[tokio::test]
async fn values() {
    let conn = connect(&url!()).await;
    assert_eq!(conn.db_info().variant(), System::Postgres);

    let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    let time = date.and_hms_micro_opt(13, 14, 15, 123456).unwrap();
    let json = JsonValue::Object(
        [("a".to_string(), JsonValue::Array(vec![JsonValue::Null]))]
            .into_iter()
            .collect(),
    );

    for (ty, value) in [
        ("bool", Value::Bool(true)),
        ("int2", Value::SmallInt(-5)),
        ("int4", Value::Int(i32::MIN)),
        ("int8", Value::BigInt(i64::MAX)),
        ("oid", Value::BigInt(4_000_000_000)),
        ("float4", Value::from(1.5f32)),
        ("float8", Value::from(-0.1f64)),
        ("text", Value::from("blåbær")),
        ("varchar(10)", Value::from("")),
        ("bytea", Value::ByteArray(vec![0, 255].into())),
        ("date", Value::Date(date)),
        ("time", Value::Time(time.time())),
        ("timestamp", Value::Timestamp(time)),
        ("timestamptz", Value::Timestamp(time)),
        ("uuid", Value::from("67e55044-10b1-426f-9247-bb680e5fe0c8")),
        ("json", Value::Json(json.clone())),
        ("jsonb", Value::Json(json)),
        ("numeric", Value::from("-12345678901234567890.0123400")),
        ("numeric", Value::from("0.00001")),
        ("numeric", Value::from("100000")),
        ("numeric", Value::from("NaN")),
        (
            "int4[]",
            Value::Array(vec![Value::Int(1), Value::Null, Value::Int(3)]),
        ),
        ("text[]", Value::Array(vec![])),
    ] {
        let sql = format!("SELECT $1::{ty}");
        let read = one(&conn, &sql, vec![value.clone()]).await.unwrap();
        match (ty, &value) {
            ("uuid", Value::Text(text)) => {
                let Value::Uuid(id) = read else {
                    panic!("{read:?}")
                };
                assert_eq!(id.to_string(), text.as_str());
            }
            _ => assert_eq!(read, value, "{ty}"),
        }
        let null = one(&conn, &sql, vec![Value::Null]).await.unwrap();
        assert_eq!(null, Value::Null, "{ty}");
    }

    // The server agrees about what the numeric is
    for text in ["-12345678901234567890.0123400", "0.00001", "1e20", "10000"] {
        let same = one(
            &conn,
            &format!("SELECT $1::numeric = '{text}'::numeric"),
            vec![text.into()],
        )
        .await
        .unwrap();
        assert_eq!(same, Value::Bool(true), "{text}");
    }

    // More than one dimension
    assert!(
        one(&conn, "SELECT ARRAY[[1,2],[3,4]]", vec![])
            .await
            .is_err()
    );
    assert!(one(&conn, "SELECT interval '1 day'", vec![]).await.is_err());
    assert_eq!(
        one(&conn, "SELECT pg_sleep(0)", vec![]).await.unwrap(),
        Value::Null
    );
}

#[tokio::test]
async fn enums_and_domains() {
    let mut conn = connect(&url!()).await;
    // Rolled back, so nothing is left behind
    let tx = conn.begin().await.unwrap();
    tx.exec_batch(
        "CREATE TYPE usql_mood AS ENUM ('ok', 'bad');
         CREATE DOMAIN usql_age AS int8 CHECK (VALUE > 0);",
    )
    .await
    .unwrap();
    let read = one(&tx, "SELECT $1::usql_mood", vec!["ok".into()])
        .await
        .unwrap();
    assert_eq!(read, Value::from("ok"));
    let read = one(&tx, "SELECT $1::usql_age", vec![3i32.into()])
        .await
        .unwrap();
    assert_eq!(read, Value::BigInt(3));
    let read = one(
        &tx,
        "SELECT $1::usql_mood[]",
        vec![Value::Array(vec!["bad".into()])],
    )
    .await
    .unwrap();
    assert_eq!(read, Value::Array(vec!["bad".into()]));
}

#[tokio::test]
async fn typed() {
    let conn = connect(&url!()).await;
    let rows = query(
        &conn,
        "SELECT 1::int8 AS a, 2.50::numeric AS b, '{\"x\":1}'::text AS c, NULL::int4 AS d,
                40000::int4 AS e, sum(n) AS f FROM (VALUES (1::int8), (2)) AS t(n)",
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
        Err(PostgresError::Convert { .. })
    ));
    assert_eq!(get("f", Type::BigInt).unwrap(), Value::BigInt(3));
    assert!(matches!(get("?", Type::Int), Err(PostgresError::NotFound)));
    assert!(matches!(
        row.get(ColumnIndex::Index(6)),
        Err(PostgresError::NotFound)
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

    // Wrong number, out of range and wrong type
    for params in [
        vec![],
        vec![Value::Int(1), Value::Int(2)],
        vec![Value::Int(40_000)],
        vec!["x".into()],
    ] {
        assert!(query(&conn, "SELECT $1::int2", params).await.is_err());
    }
    // The connection is still usable
    assert_eq!(
        one(&conn, "SELECT $1::int2", vec![1i64.into()])
            .await
            .unwrap(),
        Value::SmallInt(1)
    );

    let options = PostgresOptions::from_url("postgres://localhost");
    assert!(Postgres::create_pool(options).await.is_err());
}

#[tokio::test]
async fn transactions() {
    let mut conn = connect(&url!()).await;
    conn.exec_batch("CREATE TEMP TABLE tx(id int4)")
        .await
        .unwrap();

    for (id, commit) in [(1, Some(true)), (2, Some(false)), (3, None)] {
        let tx = conn.begin().await.unwrap();
        let mut stmt = tx.prepare("INSERT INTO tx VALUES ($1)").await.unwrap();
        tx.exec(&mut stmt, vec![Value::Int(id).into()])
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
}

#[tokio::test]
async fn pool() {
    let options = PostgresOptions::from_url(url!()).max_size(1);
    let pool = Postgres::create_pool(options).await.unwrap();
    let mut pids = Vec::new();
    for pool in [pool.clone(), pool] {
        let conn = pool.get().await.unwrap();
        pids.push(one(&conn, "SELECT pg_backend_pid()", vec![]).await.unwrap());
    }
    assert_eq!(pids[0], pids[1]);
}

#[tokio::test]
async fn postgis() {
    let conn = connect(&url!()).await;
    for text in [
        "POINT(1 2)",
        "SRID=4326;LINESTRING(1 2 3, 4 5 6)",
        "POLYGON((0 0, 1 0, 1 1, 0 0))",
        "MULTIPOINT((1 2), (3 4))",
        "MULTILINESTRING((1 2, 3 4))",
        "MULTIPOLYGON(((0 0, 1 0, 1 1, 0 0)))",
        "SRID=25832;GEOMETRYCOLLECTION(POINT(1 2), GEOMETRYCOLLECTION(LINESTRING(1 2, 3 4)))",
        "GEOMETRYCOLLECTION EMPTY",
        "POINT EMPTY",
    ] {
        let geo = Geob::from_text(text).unwrap();
        let rows = query(
            &conn,
            "SELECT $1::geometry, ST_AsEWKT($1::geometry) = ST_AsEWKT($2::text::geometry)",
            vec![Value::Geometry(geo.clone()), text.into()],
        )
        .await
        .unwrap();
        let row = &rows[0];
        let read = row
            .get_typed(ColumnIndex::Index(0), Type::Geometry(geo.kind()))
            .unwrap()
            .to_owned();
        let Value::Geometry(read) = read else {
            panic!("{read:?}")
        };
        assert_eq!(read.to_string(), geo.to_string());
        assert_eq!(read.srid(), geo.srid());
        assert_eq!(
            row.get(ColumnIndex::Index(1)).unwrap().to_owned(),
            Value::Bool(true),
            "{text}"
        );
    }

    let point = Value::Geometry(Geob::from_text("SRID=4326;POINT(12 55)").unwrap());
    let read = one(&conn, "SELECT $1::geography", vec![point.clone()])
        .await
        .unwrap();
    assert_eq!(read, point);

    let many = Value::Array(vec![point.clone(), Value::Null]);
    let read = one(&conn, "SELECT $1::geometry[]", vec![many.clone()])
        .await
        .unwrap();
    assert_eq!(read, many);

    let rows = query(&conn, "SELECT $1::geometry", vec![point])
        .await
        .unwrap();
    let line = Geob::from_text("LINESTRING(1 2, 3 4)").unwrap().kind();
    assert!(
        rows[0]
            .get_typed(ColumnIndex::Index(0), Type::Geometry(line))
            .is_err()
    );
    // Curves are not supported
    assert!(
        one(
            &conn,
            "SELECT 'CIRCULARSTRING(0 0, 1 1, 2 0)'::geometry",
            vec![]
        )
        .await
        .is_err()
    );
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

    let pool = usql::Pool::<Postgres>::open(PostgresOptions::from_url(url!()))
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

    let user = conn
        .fetch_one(
            insert("usql_user")
                .with("name", val("Rasmus"))
                // Narrower than the column
                .with("age", val(40i32))
                .returning(("id", "name", "age")),
        )
        .await
        .unwrap();
    let user = User::from_row(user).unwrap();
    assert_eq!((user.id, user.name.as_str(), user.age), (1, "Rasmus", 40));

    conn.exec(
        insert("usql_user")
            .with("name", val("Wilbur"))
            .with("age", Value::Null),
    )
    .await
    .unwrap();

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
        kind: usql_any::DatabaseConfig::Postgres(usql_any::PostgresConfig {
            url: url!(),
            max_connections: Some(1),
        }),
    };
    let pool = config.crate_pool().await.unwrap();
    let mut conn = pool.get().await.unwrap();
    assert_eq!(conn.db_info().variant(), System::Postgres);

    let tx = conn.begin().await.unwrap();
    tx.exec_batch("CREATE TEMP TABLE any_test(id int4)")
        .await
        .unwrap();
    let mut stmt = tx
        .prepare("INSERT INTO any_test VALUES ($1) RETURNING id")
        .await
        .unwrap();
    let rows: Vec<_> = tx
        .query(&mut stmt, vec![Value::Int(7).into()])
        .try_collect()
        .await
        .unwrap();
    assert_eq!(
        rows[0].get(ColumnIndex::Named("id")).unwrap().to_owned(),
        Value::Int(7)
    );
    tx.commit().await.unwrap();

    let mut stmt = conn.prepare("SELECT id FROM any_test").await.unwrap();
    let rows: Vec<_> = conn.query(&mut stmt, vec![]).try_collect().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(conn.prepare("SELECT * FROM usql_missing").await.is_err());
}

#[tokio::test]
async fn migrate() {
    let pool = Postgres::create_pool(PostgresOptions::from_url(url!()))
        .await
        .unwrap();
    let reset = "DROP TABLE IF EXISTS usql_migrations, usql_migrated";
    pool.get().await.unwrap().exec_batch(reset).await.unwrap();

    let dir = std::env::temp_dir().join(format!("usql-postgres-{}", std::process::id()));
    for (name, sql) in [
        ("1_first", "CREATE TABLE usql_migrated(id int4)"),
        ("2_second", "INSERT INTO usql_migrated VALUES (1), (2);"),
    ] {
        std::fs::create_dir_all(dir.join(name)).unwrap();
        std::fs::write(dir.join(name).join("up.postgres"), sql).unwrap();
    }

    let migrator = usql_migrate::Migrator::<Postgres, _>::new(
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

#[cfg(feature = "rustls")]
#[tokio::test]
async fn tls() {
    use usql_postgres::{
        Tls,
        rustls::{
            RootCertStore,
            pki_types::{CertificateDer, pem::PemObject},
        },
    };

    let url = url!();
    let Ok(ca) = std::env::var("USQL_POSTGRES_CA") else {
        eprintln!("skipped: USQL_POSTGRES_CA is not set");
        return;
    };
    let url = format!(
        "{url}{}sslmode=require",
        if url.contains('?') { '&' } else { '?' }
    );
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from_pem_file(ca).unwrap())
        .unwrap();

    let options = PostgresOptions::from_url(url.as_str());
    let pool = Postgres::create_pool(options.clone().tls(Tls::roots(roots).unwrap()))
        .await
        .unwrap();
    let conn = pool.get().await.unwrap();
    let ssl = "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()";
    assert_eq!(one(&conn, ssl, vec![]).await.unwrap(), Value::Bool(true));

    // Not trusted, and not encrypted
    let untrusted = Tls::roots(RootCertStore::empty()).unwrap();
    for options in [options.clone().tls(untrusted), options] {
        let pool = Postgres::create_pool(options).await.unwrap();
        assert!(pool.get().await.is_err());
    }
}
