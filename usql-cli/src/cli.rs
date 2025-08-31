use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use usql_any::AnyConnector;

use crate::{exec::ExecCli, fetch::FetchCli};

#[derive(Parser)]
pub struct Cli {
    #[clap(short, long)]
    config: Option<PathBuf>,
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Fetch(FetchCli),
    Exec(ExecCli),
}

impl Cli {
    async fn resolve_config(&self) -> anyhow::Result<usql_any::Config> {
        let content = if let Some(config) = &self.config {
            let content = tokio::fs::read_to_string(config).await?;
            content
        } else {
            tokio::fs::read_to_string("./usql.json")
                .await
                .context("No config")?
        };

        let cfg = serde_json::from_str(&content)?;

        Ok(cfg)
    }

    pub async fn run() -> anyhow::Result<()> {
        let cli = Cli::parse();

        let config = cli.resolve_config().await?;

        let pool = usql::Pool::<AnyConnector>::new(config.crate_pool().await?);

        match cli.commands {
            Commands::Fetch(fetch) => {
                fetch.run(pool).await?;
            }
            Commands::Exec(exec) => {
                exec.run(pool).await?;
            }
        }

        Ok(())
    }
}
