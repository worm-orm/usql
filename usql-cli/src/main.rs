use crate::cli::Cli;

mod cli;
mod exec;
mod fetch;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // println!(
    //     "{}",
    //     serde_json::to_string_pretty(&usql_any::Config {
    //         kind: usql_any::DatabaseConfig::Sqlite(usql_any::SqliteConfig::Path {
    //             path: "/tedt.db".into()
    //         }),
    //         workers: None
    //     })?
    // );
    Cli::run().await?;
    Ok(())
}
