use std::path::PathBuf;

use usql_migrate::{Migrator, sql::SqlLoader};
use usql_sqlite::{Sqlite, SqliteOptions};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn core::error::Error + Send + Sync>> {
    //
    let pool = usql::Pool::<Sqlite>::open(SqliteOptions::default()).await?;

    let migrator = Migrator::<Sqlite, _>::new(
        pool.clone(),
        SqlLoader::default(),
        PathBuf::from("usql-migrate/examples/migrations"),
        "migrations".to_string(),
    );

    println!("Migrations {}", migrator.has_migrations().await?);

    migrator.migrate().await?;

    let migrations = migrator.list_migrations().await?;

    println!("{:#?}", migrations);

    Ok(())
}
