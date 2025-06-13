use usql::{FromRow, core::Connector};

#[derive(Debug, FromRow)]
struct User {
    id: i32,
    name: String,
}

fn main() {
    smol::block_on(async move {
        let pool = usql_core::Sqlite::create_pool(usql_core::SqliteOptions::default())
            .await
            .unwrap();

        let core = usql::Pool::<usql_core::Sqlite>::new(pool);

        let conn = core.conn().await.unwrap();

        let row = conn.fetch_one("SELECT * FROM users").await.unwrap();

        let user = User::from_row(row).unwrap();
    });
}
