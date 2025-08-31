use std::collections::HashMap;

use anyhow::Context;
use clap::Args;
use futures::TryStreamExt;
use usql::core::Row as _;
use usql_any::AnyConnector;
use usql_value::Value;

#[derive(Args)]
pub struct ExecCli {
    path: String,
    #[clap(short, long, default_value_t = false)]
    exec: bool,
}

impl ExecCli {
    pub async fn run(self, pool: usql::Pool<AnyConnector>) -> anyhow::Result<()> {
        let script = if !self.exec {
            let script = tokio::fs::read_to_string(&self.path)
                .await
                .with_context(|| format!("Failed to read: {}", self.path))?;
            script
        } else {
            self.path
        };

        let conn = pool.conn().await?;

        conn.exec(&*script).await?;

        Ok(())
    }
}
