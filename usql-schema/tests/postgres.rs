//! Needs a database with PostGIS, and does nothing without `USQL_POSTGRES_URL`.
//! Everything is rolled back.

use usql_builder::{StatementExt, schema::ReferentialAction};
use usql_core::{Connection, Connector, Executor, Pool, System};
use usql_postgres::{Postgres, PostgresConn, PostgresOptions};
use usql_schema::{Auto, Inspector, Schema, Table, TableKind};
use usql_value::{Type, geob::GeoType};

macro_rules! connect {
    () => {
        match std::env::var("USQL_POSTGRES_URL") {
            Ok(url) => connect(&url).await,
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

fn find<'a>(schema: &'a Schema, name: &str) -> &'a Table {
    schema.tables.iter().find(|m| m.name == name).unwrap()
}

const SETUP: &str = r#"
    CREATE SCHEMA "usql ""s";
    CREATE SCHEMA usql_other;
    SET LOCAL search_path = "usql ""s", public;
    CREATE TYPE mood AS ENUM ('ok', 'bad');
    COMMENT ON TYPE mood IS 'feeling';
    CREATE DOMAIN age AS int4 CHECK (VALUE > 0);
    CREATE TABLE usql_other.parent(id int PRIMARY KEY, other int);
    CREATE TABLE parent(
        id serial PRIMARY KEY,
        big bigint GENERATED ALWAYS AS IDENTITY,
        name varchar(10) NOT NULL DEFAULT 'it''s',
        mood mood, age age, tags text[], amount numeric(10, 2),
        at timestamptz DEFAULT now(),
        twice int GENERATED ALWAYS AS (id * 2) STORED,
        geo geometry(Point, 4326),
        gone int,
        CONSTRAINT name_key UNIQUE (name, id)
    );
    ALTER TABLE parent DROP COLUMN gone;
    COMMENT ON TABLE parent IS 'a table';
    COMMENT ON COLUMN parent.name IS 'a column';
    CREATE INDEX geo_idx ON parent USING gist (geo);
    CREATE INDEX plain_idx ON parent (age, "name");
    CREATE INDEX odd_idx ON parent (lower(name), id DESC) INCLUDE (age) WHERE age > 1;
    CREATE INDEX ops_idx ON parent (name varchar_pattern_ops);
    CREATE TABLE child(
        a int, b int, "odd ""c" int REFERENCES parent ON DELETE CASCADE,
        o int REFERENCES usql_other.parent,
        CONSTRAINT child_key PRIMARY KEY (b, a)
    );
    CREATE TABLE grandchild(
        x int, y int, CONSTRAINT xy FOREIGN KEY (x, y) REFERENCES child (b, a) ON UPDATE SET NULL
    );
    CREATE TABLE part(id int, PRIMARY KEY (id)) PARTITION BY RANGE (id);
    CREATE TABLE part_1 PARTITION OF part FOR VALUES FROM (1) TO (10);
    CREATE TABLE to_part(id int REFERENCES part);
    CREATE VIEW v AS SELECT id, name FROM parent;
    CREATE MATERIALIZED VIEW mv AS SELECT 1 AS one;
    CREATE FUNCTION trig() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RETURN NEW; END';
    CREATE TRIGGER trig BEFORE INSERT ON child FOR EACH ROW EXECUTE FUNCTION trig();
"#;

#[tokio::test]
async fn inspect() {
    let mut conn = connect!();
    let tx = conn.begin().await.unwrap();
    tx.exec_batch(SETUP).await.unwrap();

    let schema = Inspector::new(&tx).inspect().await.unwrap();
    assert_eq!(schema.name, "usql \"s");
    let names: Vec<_> = schema.tables.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        ["child", "grandchild", "parent", "part", "part_1", "to_part"]
    );

    let parent = find(&schema, "parent");
    let col = |name: &str| parent.columns.iter().find(|m| m.name == name).unwrap();
    assert_eq!(parent.columns.len(), 10);
    assert_eq!(parent.comment.as_deref(), Some("a table"));
    assert_eq!(col("name").comment.as_deref(), Some("a column"));
    assert_eq!(col("id").auto, Some(Auto::Serial));
    assert_eq!(col("id").default, None);
    assert_eq!(col("big").auto, Some(Auto::Identity));
    assert_eq!(col("name").type_name, "character varying(10)");
    assert!(!col("name").nullable && col("mood").nullable);
    assert!(col("name").default.as_ref().unwrap().starts_with("'it''s'"));
    assert_eq!(col("mood").ty, Some(Type::Text));
    assert_eq!(col("age").ty, Some(Type::Int));
    assert_eq!(col("tags").ty, Some(Type::Array(Type::Text.into())));
    assert_eq!(col("at").ty, Some(Type::DateTime));
    assert_eq!(col("geo").ty, Some(Type::Geometry(GeoType::Point)));
    let twice = col("twice").generated.as_ref().unwrap();
    assert!(twice.stored && twice.expression.is_some() && col("twice").default.is_none());
    assert_eq!(parent.uniques[0].columns, ["name", "id"]);
    assert_eq!(parent.uniques[0].name.as_deref(), Some("name_key"));

    let names: Vec<_> = parent.indexes.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["geo_idx", "odd_idx", "ops_idx", "plain_idx"]);
    assert_eq!(parent.indexes[0].method, "gist");
    let odd = &parent.indexes[1];
    assert!(odd.partial && odd.predicate.is_some());
    assert!(odd.parts[0].expression.as_ref().unwrap().contains("lower"));
    assert_eq!(odd.parts[1].column.as_deref(), Some("id"));
    assert!(odd.parts[1].descending && !odd.parts[0].descending);
    assert_eq!(odd.included, ["age"]);

    let child = find(&schema, "child");
    assert_eq!(child.primary_key.as_ref().unwrap().columns, ["b", "a"]);
    assert_eq!(child.triggers.len(), 1);
    assert_eq!(child.foreign_keys.len(), 2);
    let fk = |col: &str| {
        child
            .foreign_keys
            .iter()
            .find(|m| m.columns == [col])
            .unwrap()
    };
    assert_eq!(fk("odd \"c").ref_schema, "usql \"s");
    assert_eq!(fk("odd \"c").on_delete, ReferentialAction::Cascade);
    assert_eq!(fk("o").ref_schema, "usql_other");
    let xy = &find(&schema, "grandchild").foreign_keys[0];
    assert_eq!(xy.columns, ["x", "y"]);
    assert_eq!(xy.ref_columns, ["b", "a"]);
    assert_eq!(xy.on_update, ReferentialAction::SetNull);

    assert_eq!(find(&schema, "part").kind, TableKind::Partitioned);
    assert_eq!(
        find(&schema, "part_1").partition_of.as_deref(),
        Some("part")
    );
    assert_eq!(find(&schema, "to_part").foreign_keys.len(), 1);

    let views: Vec<_> = schema
        .views
        .iter()
        .map(|m| (m.name.as_str(), m.materialized))
        .collect();
    assert_eq!(views, [("mv", true), ("v", false)]);
    assert_eq!(schema.views[1].columns.len(), 2);
    assert!(
        schema.views[1]
            .definition
            .as_ref()
            .unwrap()
            .contains("SELECT")
    );

    assert_eq!(schema.enums.len(), 1);
    assert_eq!(schema.enums[0].values, ["ok", "bad"]);
    assert_eq!(schema.enums[0].comment.as_deref(), Some("feeling"));

    let inspector = Inspector::new(&tx);
    assert_eq!(
        inspector.table("child").await.unwrap().as_ref(),
        Some(child)
    );
    assert!(inspector.table("missing").await.unwrap().is_none());
    assert!(
        inspector
            .schemas()
            .await
            .unwrap()
            .contains(&"usql \"s".to_string())
    );
    let other = Inspector::new(&tx).schema("usql_other");
    let other = other.table("parent").await.unwrap().unwrap();
    assert_eq!(other.columns.len(), 2);

    // The tables of PostGIS are not included
    let public = Inspector::new(&tx).schema("public").tables().await.unwrap();
    assert!(public.iter().all(|m| m.name != "spatial_ref_sys"));

    let conversion = parent.to_builder(System::Postgres);
    let omitted: Vec<_> = conversion.omitted.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        omitted,
        [
            "parent", "big", "name", "twice", "name_key", "geo_idx", "odd_idx", "ops_idx"
        ]
    );
    assert_eq!(conversion.indexes.len(), 1);
}

#[tokio::test]
async fn round_trip() {
    let mut conn = connect!();
    let tx = conn.begin().await.unwrap();
    tx.exec_batch(
        r#"CREATE SCHEMA usql_a; CREATE SCHEMA usql_b; SET LOCAL search_path = usql_a, public;
        CREATE TABLE p(id bigserial PRIMARY KEY, name varchar(10) NOT NULL DEFAULT 'it''s',
                       geo geometry(Point, 4326), tags text[] DEFAULT '{}', "odd ""c" int);
        CREATE TABLE q(a smallint NOT NULL, b uuid NOT NULL, p bigint REFERENCES p ON DELETE CASCADE,
                       n float8 DEFAULT (1 + 2), CONSTRAINT my_key PRIMARY KEY (b, a));
        CREATE UNIQUE INDEX qi ON q (p, n);
        CREATE INDEX pi ON p ("odd ""c");"#,
    )
    .await
    .unwrap();
    let before = Inspector::new(&tx).inspect().await.unwrap();

    tx.exec_batch("SET LOCAL search_path = usql_b, public")
        .await
        .unwrap();
    for table in &before.tables {
        let conversion = table.to_builder(System::Postgres);
        assert_eq!(conversion.omitted, []);
        let mut sql = vec![conversion.table.to_sql(System::Postgres).unwrap()];
        for index in conversion.indexes {
            sql.push(index.to_sql(System::Postgres).unwrap());
        }
        for sql in sql {
            assert!(sql.bindings.is_empty());
            tx.exec_batch(&sql.sql).await.unwrap();
        }
    }

    let mut after = Inspector::new(&tx).inspect().await.unwrap();
    assert_eq!(after.name, "usql_b");
    after.name = before.name.clone();
    for (a, b) in after.tables.iter_mut().zip(&before.tables) {
        a.schema = b.schema.clone();
        for fk in &mut a.foreign_keys {
            assert_eq!(fk.ref_schema, "usql_b");
            fk.ref_schema = "usql_a".to_string();
        }
        for (a, b) in a.indexes.iter_mut().zip(&b.indexes) {
            a.definition = b.definition.clone();
        }
    }
    assert_eq!(after, before);
}

#[tokio::test]
async fn any() {
    let Ok(url) = std::env::var("USQL_POSTGRES_URL") else {
        eprintln!("skipped: USQL_POSTGRES_URL is not set");
        return;
    };
    let options = PostgresOptions::from_url(url);
    let pool = usql_any::AnyConnector::create_pool(options.into())
        .await
        .unwrap();
    let mut conn = pool.get().await.unwrap();
    let tx = conn.begin().await.unwrap();
    tx.exec_batch(SETUP).await.unwrap();
    let schema = Inspector::new(&tx).inspect().await.unwrap();
    assert_eq!(schema.tables.len(), 6);
    assert_eq!(find(&schema, "parent").indexes.len(), 4);
}
