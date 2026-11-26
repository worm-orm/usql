//! Needs a user that can create databases, and does nothing without `USQL_MYSQL_URL`.
//! MySQL can't roll back DDL, so each test makes and drops its own databases.

use futures::TryStreamExt;
use usql_builder::{StatementExt, schema::ReferentialAction};
use usql_core::{Connector, Executor, Pool, Row, System};
use usql_mysql::{Mysql, MysqlConn, MysqlOptions};
use usql_schema::{Auto, Inspector, Schema, Table, TableKind};
use usql_value::{Type, geob::GeoType};

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

fn find<'a>(schema: &'a Schema, name: &str) -> &'a Table {
    schema.tables.iter().find(|m| m.name == name).unwrap()
}

const SETUP: &str = r#"
    DROP DATABASE IF EXISTS `usql_i's`;
    DROP DATABASE IF EXISTS usql_i_other;
    CREATE DATABASE `usql_i's`;
    CREATE DATABASE usql_i_other;
    USE `usql_i's`;
    CREATE TABLE usql_i_other.parent(id int PRIMARY KEY, other int);
    CREATE TABLE parent(
        id int unsigned NOT NULL AUTO_INCREMENT PRIMARY KEY,
        name varchar(10) NOT NULL DEFAULT 'it''s' COMMENT 'a column',
        mood enum('ok', 'bad'),
        flag boolean NOT NULL DEFAULT TRUE,
        amount decimal(10, 2) DEFAULT 1.50,
        at datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
        expr varchar(20) DEFAULT (concat('a', 'b')),
        twice decimal(12, 2) GENERATED ALWAYS AS (amount * 2) STORED,
        geo point,
        meta json,
        UNIQUE KEY name_key (name, id),
        INDEX prefix_idx (name(3), id DESC),
        INDEX fn_idx ((lower(name))),
        FULLTEXT INDEX ft (name)
    ) COMMENT 'a table';
    CREATE TABLE child(
        a int, b int, `odd ``c` int unsigned, o int,
        PRIMARY KEY (b, a),
        FOREIGN KEY (`odd ``c`) REFERENCES parent(id) ON DELETE CASCADE,
        CONSTRAINT to_other FOREIGN KEY (o) REFERENCES usql_i_other.parent(id)
    );
    CREATE TABLE grandchild(
        x int, y int,
        CONSTRAINT xy FOREIGN KEY (x, y) REFERENCES child (b, a) ON UPDATE SET NULL
    );
    CREATE TABLE part(id int PRIMARY KEY)
        PARTITION BY RANGE (id) (PARTITION p0 VALUES LESS THAN (10));
    CREATE VIEW v AS SELECT id, name FROM parent;
    CREATE TRIGGER trig BEFORE INSERT ON child FOR EACH ROW SET NEW.a = NEW.a + 1;
"#;

const TEARDOWN: &str = "DROP DATABASE `usql_i's`; DROP DATABASE usql_i_other";

async fn is_mariadb(conn: &MysqlConn) -> bool {
    let mut stmt = conn.prepare("SELECT VERSION()").await.unwrap();
    let rows: Vec<_> = conn.query(&mut stmt, vec![]).try_collect().await.unwrap();
    let version = rows[0].get(0usize.into()).unwrap().to_owned();
    version.try_get::<String>().unwrap().contains("MariaDB")
}

#[tokio::test]
async fn inspect() {
    let conn = connect(&url!()).await;
    // Which has no indexes on expressions, and other metadata
    if is_mariadb(&conn).await {
        eprintln!("skipped: MySQL only");
        return;
    }
    conn.exec_batch(SETUP).await.unwrap();

    let schema = Inspector::new(&conn).inspect().await.unwrap();
    assert_eq!(schema.name, "usql_i's");
    let names: Vec<_> = schema.tables.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["child", "grandchild", "parent", "part"]);

    let parent = find(&schema, "parent");
    assert_eq!(parent.system, System::Mysql);
    let col = |name: &str| parent.columns.iter().find(|m| m.name == name).unwrap();
    assert_eq!(parent.columns.len(), 10);
    assert_eq!(parent.comment.as_deref(), Some("a table"));
    assert_eq!(col("name").comment.as_deref(), Some("a column"));
    assert_eq!(col("mood").comment, None);
    assert_eq!(col("id").auto, Some(Auto::AutoIncrement));
    assert_eq!(col("id").default, None);
    assert_eq!(col("id").ty, Some(Type::BigInt));
    assert_eq!(col("name").type_name, "varchar(10)");
    assert!(!col("name").nullable && col("mood").nullable);
    assert_eq!(col("name").default.as_deref(), Some("'it''s'"));
    assert_eq!(col("mood").type_name, "enum('ok','bad')");
    assert_eq!(col("mood").ty, Some(Type::Text));
    assert_eq!(col("flag").ty, Some(Type::Bool));
    assert_eq!(col("flag").default.as_deref(), Some("1"));
    assert_eq!(col("amount").ty, Some(Type::Text));
    assert_eq!(col("amount").default.as_deref(), Some("1.50"));
    assert_eq!(col("at").ty, Some(Type::DateTime));
    assert_eq!(col("at").default.as_deref(), Some("CURRENT_TIMESTAMP"));
    assert_eq!(col("at").on_update.as_deref(), Some("CURRENT_TIMESTAMP"));
    assert_eq!(col("name").on_update, None);
    assert_eq!(
        col("expr").default.as_deref(),
        Some("concat(_utf8mb4'a',_utf8mb4'b')")
    );
    assert_eq!(col("geo").ty, Some(Type::Geometry(GeoType::Point)));
    assert_eq!(col("meta").ty, Some(Type::Json));
    let twice = col("twice").generated.as_ref().unwrap();
    assert!(twice.stored && twice.expression.as_ref().unwrap().contains("`amount`"));
    assert_eq!(col("twice").default, None);
    assert_eq!(parent.primary_key.as_ref().unwrap().columns, ["id"]);
    assert!(parent.uniques.is_empty());

    let names: Vec<_> = parent.indexes.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["fn_idx", "ft", "name_key", "prefix_idx"]);
    let index = |name: &str| parent.indexes.iter().find(|m| m.name == name).unwrap();
    assert!(
        index("fn_idx").parts[0]
            .expression
            .as_ref()
            .unwrap()
            .contains("lower")
    );
    assert_eq!(index("ft").method, "fulltext");
    assert!(index("name_key").unique && index("name_key").method == "btree");
    assert_eq!(index("name_key").parts.len(), 2);
    let prefix = &index("prefix_idx").parts;
    assert_eq!(prefix[0].column, None);
    assert_eq!(prefix[0].expression.as_deref(), Some("`name`(3)"));
    assert_eq!(prefix[1].column.as_deref(), Some("id"));
    assert!(prefix[1].descending && !prefix[0].descending);

    let child = find(&schema, "child");
    assert_eq!(child.primary_key.as_ref().unwrap().columns, ["b", "a"]);
    assert_eq!(child.triggers.len(), 1);
    assert!(
        child.triggers[0]
            .definition
            .contains("BEFORE INSERT ON `child`")
    );
    assert_eq!(child.foreign_keys.len(), 2);
    // The indexes made for the foreign keys are left out
    assert_eq!(child.indexes, []);
    let fk = |col: &str| {
        child
            .foreign_keys
            .iter()
            .find(|m| m.columns == [col])
            .unwrap()
    };
    assert_eq!(fk("odd `c").name.as_deref(), Some("child_ibfk_1"));
    assert_eq!(fk("odd `c").ref_schema, "usql_i's");
    assert_eq!(fk("odd `c").ref_table, "parent");
    assert_eq!(fk("odd `c").on_delete, ReferentialAction::Cascade);
    assert_eq!(fk("o").ref_schema, "usql_i_other");
    let grandchild = find(&schema, "grandchild");
    let xy = &grandchild.foreign_keys[0];
    assert_eq!(xy.columns, ["x", "y"]);
    assert_eq!(xy.ref_columns, ["b", "a"]);
    assert_eq!(xy.on_update, ReferentialAction::SetNull);
    assert_eq!(grandchild.indexes, []);

    assert_eq!(find(&schema, "part").kind, TableKind::Partitioned);

    assert_eq!(schema.views.len(), 1);
    assert_eq!(schema.views[0].name, "v");
    assert_eq!(schema.views[0].columns.len(), 2);
    assert!(
        schema.views[0]
            .definition
            .as_ref()
            .unwrap()
            .contains("select")
    );
    assert!(schema.enums.is_empty());

    let inspector = Inspector::new(&conn);
    assert_eq!(
        inspector.table("child").await.unwrap().as_ref(),
        Some(child)
    );
    assert!(inspector.table("missing").await.unwrap().is_none());
    let schemas = inspector.schemas().await.unwrap();
    assert!(schemas.contains(&"usql_i's".to_string()));
    assert!(!schemas.contains(&"mysql".to_string()));
    let other = Inspector::new(&conn).schema("usql_i_other");
    let other = other.table("parent").await.unwrap().unwrap();
    assert_eq!(other.columns.len(), 2);

    let conversion = parent.to_builder(System::Mysql);
    let omitted: Vec<_> = conversion.omitted.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        omitted,
        [
            "parent",
            "id",
            "name",
            "at",
            "twice",
            "fn_idx",
            "ft",
            "prefix_idx"
        ]
    );
    assert_eq!(conversion.indexes.len(), 1);

    conn.exec_batch(TEARDOWN).await.unwrap();
}

#[tokio::test]
async fn round_trip() {
    let conn = connect(&url!()).await;
    conn.exec_batch(
        r#"DROP DATABASE IF EXISTS usql_rt_a; DROP DATABASE IF EXISTS usql_rt_b;
        CREATE DATABASE usql_rt_a; CREATE DATABASE usql_rt_b; USE usql_rt_a;
        CREATE TABLE p(id bigint AUTO_INCREMENT PRIMARY KEY,
                       name varchar(10) NOT NULL DEFAULT 'it''s', flag boolean DEFAULT TRUE,
                       `odd ``c` int, body text DEFAULT ('x'),
                       at datetime(6) DEFAULT CURRENT_TIMESTAMP(6), meta json);
        CREATE TABLE q(a smallint NOT NULL, b binary(16) NOT NULL, p bigint,
                       n double DEFAULT (1 + 2), PRIMARY KEY (b, a),
                       FOREIGN KEY (p) REFERENCES p(id) ON DELETE CASCADE);
        CREATE UNIQUE INDEX qi ON q (p, n);
        CREATE INDEX pi ON p (`odd ``c`);"#,
    )
    .await
    .unwrap();
    let before = Inspector::new(&conn).inspect().await.unwrap();
    assert_eq!(before.tables.len(), 2);

    conn.exec_batch("USE usql_rt_b").await.unwrap();
    for table in &before.tables {
        let conversion = table.to_builder(System::Mysql);
        assert_eq!(conversion.omitted, []);
        let mut sql = vec![conversion.table.to_sql(System::Mysql).unwrap()];
        for index in conversion.indexes {
            sql.push(index.to_sql(System::Mysql).unwrap());
        }
        for sql in sql {
            assert!(sql.bindings.is_empty());
            conn.exec_batch(&sql.sql).await.unwrap();
        }
    }

    let mut after = Inspector::new(&conn).inspect().await.unwrap();
    assert_eq!(after.name, "usql_rt_b");
    after.name = before.name.clone();
    for (a, b) in after.tables.iter_mut().zip(&before.tables) {
        a.schema = b.schema.clone();
        for fk in &mut a.foreign_keys {
            assert_eq!(fk.ref_schema, "usql_rt_b");
            fk.ref_schema = "usql_rt_a".to_string();
        }
    }
    assert_eq!(after, before);

    conn.exec_batch("DROP DATABASE usql_rt_a; DROP DATABASE usql_rt_b")
        .await
        .unwrap();
}

#[tokio::test]
async fn any() {
    let url = url!();
    let pool = usql_any::AnyConnector::create_pool(MysqlOptions::from_url(url).into())
        .await
        .unwrap();
    let conn = pool.get().await.unwrap();
    conn.exec_batch(
        "DROP DATABASE IF EXISTS usql_any; CREATE DATABASE usql_any; USE usql_any;
         CREATE TABLE t(id int PRIMARY KEY, n varchar(5), KEY n_idx (n))",
    )
    .await
    .unwrap();
    let schema = Inspector::new(&conn).inspect().await.unwrap();
    assert_eq!(schema.name, "usql_any");
    assert_eq!(schema.tables.len(), 1);
    assert_eq!(schema.tables[0].indexes.len(), 1);
    conn.exec_batch("DROP DATABASE usql_any").await.unwrap();
}
