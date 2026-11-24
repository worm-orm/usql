use anyhow::bail;
use clap::Args;
use usql_any::AnyConnector;
use usql_schema::Inspector;

#[derive(Args)]
pub struct SchemaCli {
    /// Only this table
    table: Option<String>,
    #[clap(short, long)]
    schema: Option<String>,
}

impl SchemaCli {
    pub async fn run(self, pool: usql::Pool<AnyConnector>) -> anyhow::Result<()> {
        let conn = pool.conn().await?.into_inner();

        let mut inspector = Inspector::new(&conn);
        if let Some(schema) = self.schema {
            inspector = inspector.schema(schema);
        }

        let output = match self.table {
            Some(name) => match inspector.table(&name).await? {
                Some(table) => serde_json::to_string_pretty(&table)?,
                None => bail!("Table not found: {name}"),
            },
            None => serde_json::to_string_pretty(&inspector.inspect().await?)?,
        };

        println!("{output}");

        Ok(())
    }
}
