use std::collections::HashMap;

use anyhow::Context;
use clap::Args;
use futures::TryStreamExt;
use usql::core::Row as _;
use usql_any::AnyConnector;
use usql_value::Value;

#[derive(Args)]
pub struct FetchCli {
    path: String,
    #[clap(short, long, default_value_t = false)]
    exec: bool,
}

impl FetchCli {
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

        let mut stream = conn.fetch(&*script).await?;

        let mut results = Vec::new();

        while let Some(row) = stream.try_next().await? {
            let row = row.into_inner();

            let mut entry = HashMap::<String, Value>::default();

            for i in 0..row.len() {
                let value = row.get(i.into())?.to_owned();
                let name = row.column_name(i).unwrap();
                entry.insert(name.to_string(), value);
            }

            results.push(entry);
        }

        println!("{}", serde_json::to_string_pretty(&results)?);

        Ok(())
    }
}
