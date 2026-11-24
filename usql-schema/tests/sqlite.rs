use usql_builder::{StatementExt, schema::ReferentialAction};
use usql_core::{Connection, Connector, Executor, Pool, System};
use usql_schema::{Auto, Inspector, Table, TableKind};
use usql_sqlite::{Sqlite, SqliteConn, SqliteOptions};
use usql_value::Type;

async fn open(sql: &str) -> SqliteConn {
    let pool = Sqlite::create_pool(SqliteOptions::default()).await.unwrap();
    let conn = pool.get().await.unwrap();
    conn.exec_batch(sql).await.unwrap();
    conn
}

async fn table<E: Executor<Connector = Sqlite>>(conn: &E, name: &str) -> Table {
    Inspector::new(conn).table(name).await.unwrap().unwrap()
}

#[tokio::test]
async fn empty() {
    let conn = open("").await;
    let schema = Inspector::new(&conn).inspect().await.unwrap();
    assert_eq!(schema.name, "main");
    assert!(schema.tables.is_empty() && schema.views.is_empty() && schema.enums.is_empty());
    assert!(
        Inspector::new(&conn)
            .table("missing")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn columns() {
    let conn = open(
        "CREATE TABLE t(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name VARCHAR(10) NOT NULL DEFAULT 'it''s',
            n REAL DEFAULT (1 + 2),
            at DATETIME DEFAULT CURRENT_TIMESTAMP,
            untyped DEFAULT NULL,
            twice INT GENERATED ALWAYS AS (id * 2) STORED,
            thrice INT AS (id * 3)
        );
        CREATE TABLE fake(id INTEGER PRIMARY KEY /* AUTOINCREMENT */, b DEFAULT 'AUTOINCREMENT');
        CREATE TABLE wr(a TEXT, b INT, PRIMARY KEY (b, a)) WITHOUT ROWID;",
    )
    .await;

    let t = table(&conn, "T").await;
    assert_eq!(t.name, "t");
    let col = |name: &str| t.columns.iter().find(|m| m.name == name).unwrap();
    assert_eq!(t.columns.len(), 7);
    assert_eq!(col("id").auto, Some(Auto::AutoIncrement));
    assert!(!col("id").nullable && !col("name").nullable && col("n").nullable);
    assert_eq!(col("name").type_name, "VARCHAR(10)");
    assert_eq!(col("name").ty, Some(Type::Text));
    assert_eq!(col("name").default.as_deref(), Some("'it''s'"));
    assert_eq!(col("n").default.as_deref(), Some("1 + 2"));
    assert_eq!(col("at").ty, Some(Type::DateTime));
    assert_eq!(col("untyped").default, None);
    assert_eq!(col("untyped").type_name, "");
    assert!(col("twice").generated.as_ref().unwrap().stored);
    assert!(!col("thrice").generated.as_ref().unwrap().stored);
    assert_eq!(t.primary_key.as_ref().unwrap().columns, ["id"]);

    let fake = table(&conn, "fake").await;
    assert_eq!(fake.columns[0].auto, None);
    assert!(!fake.columns[0].nullable);

    let wr = table(&conn, "wr").await;
    assert!(wr.without_rowid);
    assert_eq!(wr.primary_key.unwrap().columns, ["b", "a"]);
    assert!(wr.columns.iter().all(|m| !m.nullable));
}

#[tokio::test]
async fn keys_and_indexes() {
    let conn = open(
        "CREATE TABLE parent(a INT, b INT, c TEXT UNIQUE, PRIMARY KEY (a, b), UNIQUE (b, c));
         CREATE TABLE child(
            x INT, y INT, z TEXT REFERENCES parent(c) ON DELETE SET NULL,
            FOREIGN KEY (x, y) REFERENCES parent ON UPDATE CASCADE
         );
         CREATE INDEX plain ON child(x, y);
         CREATE UNIQUE INDEX uniq ON child(y);
         CREATE INDEX part ON child(x) WHERE y > 1;
         CREATE INDEX expr ON child(lower(z), x DESC, z COLLATE NOCASE);
         CREATE TRIGGER trig AFTER INSERT ON child BEGIN SELECT 1; END;",
    )
    .await;

    let parent = table(&conn, "parent").await;
    let uniques: Vec<_> = parent.uniques.iter().map(|m| m.columns.clone()).collect();
    assert!(uniques.contains(&vec!["c".into()]) && uniques.contains(&vec!["b".into(), "c".into()]));
    assert!(parent.indexes.is_empty());

    let child = table(&conn, "child").await;
    let [z, xy] = child.foreign_keys.as_slice() else {
        panic!("{:?}", child.foreign_keys)
    };
    assert_eq!(
        (z.columns[0].as_str(), z.ref_columns[0].as_str()),
        ("z", "c")
    );
    assert_eq!(z.on_delete, ReferentialAction::SetNull);
    assert_eq!(xy.columns, ["x", "y"]);
    assert_eq!(xy.ref_columns, ["a", "b"]);
    assert_eq!(xy.on_update, ReferentialAction::Cascade);

    let names: Vec<_> = child.indexes.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["expr", "part", "plain", "uniq"]);
    let expr = &child.indexes[0];
    assert_eq!(expr.parts[0].column, None);
    assert!(expr.parts[1].descending);
    assert_eq!(expr.parts[2].collation.as_deref(), Some("NOCASE"));
    assert!(expr.definition.as_ref().unwrap().contains("lower(z)"));
    assert!(child.indexes[1].partial && !child.indexes[2].partial);
    assert!(child.indexes[3].unique && !child.indexes[2].unique);
    assert_eq!(child.triggers[0].name, "trig");

    let conversion = child.to_builder(System::Sqlite);
    let omitted: Vec<_> = conversion.omitted.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(omitted, ["x,y", "trig", "expr", "part"]);
    assert_eq!(conversion.indexes.len(), 2);
}

#[tokio::test]
async fn objects() {
    let conn = open(
        r#"CREATE TABLE "odd ""name'"("a ""b" INT);
        CREATE INDEX "i ""x" ON "odd ""name'"("a ""b");
        CREATE VIRTUAL TABLE search USING fts5(body);
        CREATE VIEW v AS SELECT 1 AS one;
        CREATE TRIGGER vt INSTEAD OF INSERT ON v BEGIN SELECT 1; END;
        ATTACH ':memory:' AS "other ""db";
        CREATE TABLE "other ""db".t(id INT);
        CREATE INDEX "other ""db".ti ON t(id);"#,
    )
    .await;

    let inspector = Inspector::new(&conn);
    let names = inspector.tables().await.unwrap();
    let names: Vec<_> = names.iter().map(|m| (m.name.as_str(), m.kind)).collect();
    // No shadow tables
    assert_eq!(
        names,
        [
            ("odd \"name'", TableKind::Ordinary),
            ("search", TableKind::Virtual)
        ]
    );
    let odd = table(&conn, "odd \"name'").await;
    assert_eq!(odd.columns[0].name, "a \"b");
    assert_eq!(odd.indexes[0].parts[0].column.as_deref(), Some("a \"b"));
    assert_eq!(table(&conn, "search").await.columns.len(), 1);

    let views = inspector.views().await.unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].columns[0].name, "one");
    assert_eq!(views[0].triggers[0].name, "vt");

    assert!(
        inspector
            .schemas()
            .await
            .unwrap()
            .contains(&"other \"db".to_string())
    );
    let other = Inspector::new(&conn)
        .schema("other \"db")
        .inspect()
        .await
        .unwrap();
    assert_eq!(other.tables.len(), 1);
    assert_eq!(other.tables[0].indexes[0].name, "ti");
}

#[tokio::test]
async fn round_trip() {
    let conn = open(
        "CREATE TABLE p(id INTEGER PRIMARY KEY AUTOINCREMENT, name VARCHAR(10) NOT NULL DEFAULT 'it''s');
         CREATE TABLE c(a INT NOT NULL, b TEXT NOT NULL, p INT REFERENCES p(id) ON DELETE CASCADE,
                        n REAL DEFAULT (1 + 2), PRIMARY KEY (a, b));
         CREATE UNIQUE INDEX ci ON c(p, n);",
    )
    .await;
    let before = Inspector::new(&conn).inspect().await.unwrap();

    let mut new = open("").await;
    let tx = new.begin().await.unwrap();
    for name in ["p", "c"] {
        let conversion = table(&conn, name).await.to_builder(System::Sqlite);
        assert_eq!(conversion.omitted, []);
        let mut sql = vec![conversion.table.to_sql(System::Sqlite).unwrap()];
        for index in conversion.indexes {
            sql.push(index.to_sql(System::Sqlite).unwrap());
        }
        for sql in sql {
            assert!(sql.bindings.is_empty());
            tx.exec_batch(&sql.sql).await.unwrap();
        }
    }
    let mut after = Inspector::new(&tx).inspect().await.unwrap();
    for (a, b) in after.tables.iter_mut().zip(&before.tables) {
        for (a, b) in a.indexes.iter_mut().zip(&b.indexes) {
            a.definition = b.definition.clone();
        }
    }
    assert_eq!(after, before);
}

#[tokio::test]
async fn to_postgres() {
    let conn = open(
        "CREATE TABLE t(id INTEGER PRIMARY KEY AUTOINCREMENT, a TEXT DEFAULT 'x',
                        b DATETIME DEFAULT CURRENT_TIMESTAMP, c WEIRD)",
    )
    .await;
    let conversion = table(&conn, "t").await.to_builder(System::Postgres);
    let sql = conversion.table.to_sql(System::Postgres).unwrap().sql;
    assert!(sql.contains(r#""id" BIGSERIAL PRIMARY KEY"#), "{sql}");
    assert!(sql.contains(r#""a" TEXT DEFAULT ('x')"#), "{sql}");
    let omitted: Vec<_> = conversion.omitted.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(omitted, ["b", "c"]);
}
