use std::path::PathBuf;

use usql_core::{ColumnIndex, Connector, Executor, Pool, Row};
use usql_migrate::{Migrator, sql::SqlLoader};
use usql_sqlite::{Sqlite, SqliteError, SqliteOptions};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn core::error::Error + Send + Sync>> {
    //
    let pool = Sqlite::create_pool(SqliteOptions::default()).await?;

    let migrator = Migrator::<Sqlite, _>::new(
        pool.clone(),
        SqlLoader::default(),
        PathBuf::from("usql-migrate/examples/migrations"),
        "migrations".to_string(),
    );

    println!("Migrations {}", migrator.has_migrations().await?);

    migrator.migrate().await?;

    let conn = pool.get().await?;

    let mut stmt = conn.prepare("SELECT * FROM migrations").await?;

    let mut stream = conn.query(&mut stmt, vec![]);

    while let Some(next) = usql_core::util::next(&mut stream).await {
        let next = next?;
        println!("{:?}", Row::get(&next, ColumnIndex::named("date")));
    }

    Ok(())
}
