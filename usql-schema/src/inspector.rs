use usql_core::{Connector, DatabaseInfo, Executor, System};

use crate::{
    error::Error,
    model::{Enum, Schema, Table, TableName, View},
    mysql, postgres, sqlite,
};

/// Uses several queries, so run it in a transaction for a consistent result
pub struct Inspector<'a, E> {
    executor: &'a E,
    schema: Option<String>,
}

macro_rules! dispatch {
    ($self: ident, $func: ident ( $($arg: expr),* )) => {{
        let schema = $self.schema_name().await?;
        match $self.system() {
            System::Sqlite | System::LibSql => {
                sqlite::$func($self.executor, &schema, $($arg),*).await
            }
            System::Postgres => postgres::$func($self.executor, &schema, $($arg),*).await,
            System::Mysql => mysql::$func($self.executor, &schema, $($arg),*).await,
        }
    }};
}

type Result<T, E> = core::result::Result<T, Error<<E as Executor>::Connector>>;

impl<'a, E: Executor> Inspector<'a, E> {
    pub fn new(executor: &'a E) -> Inspector<'a, E> {
        Inspector {
            executor,
            schema: None,
        }
    }

    /// Defaults to `main` on SQLite, the current schema on Postgres and the current
    /// database on MySQL
    pub fn schema(mut self, name: impl Into<String>) -> Self {
        self.schema = Some(name.into());
        self
    }

    fn system(&self) -> System {
        let info: <E::Connector as Connector>::Info = self.executor.db_info();
        info.variant()
    }

    async fn schema_name(&self) -> Result<String, E> {
        match (&self.schema, self.system()) {
            (Some(name), _) => Ok(name.clone()),
            (None, System::Sqlite | System::LibSql) => Ok("main".to_string()),
            (None, System::Postgres) => postgres::current_schema(self.executor).await,
            (None, System::Mysql) => mysql::current_schema(self.executor).await,
        }
    }

    pub async fn schemas(&self) -> Result<Vec<String>, E> {
        match self.system() {
            System::Sqlite | System::LibSql => sqlite::schemas(self.executor).await,
            System::Postgres => postgres::schemas(self.executor).await,
            System::Mysql => mysql::schemas(self.executor).await,
        }
    }

    /// The names of the tables
    pub async fn tables(&self) -> Result<Vec<TableName>, E> {
        dispatch!(self, names())
    }

    pub async fn table(&self, name: &str) -> Result<Option<Table>, E> {
        let tables: Vec<Table> = dispatch!(self, tables(Some(name)))?;
        Ok(tables.into_iter().next())
    }

    pub async fn views(&self) -> Result<Vec<View>, E> {
        dispatch!(self, views())
    }

    /// Always empty on SQLite and MySQL, where an enum is part of the column type
    pub async fn enums(&self) -> Result<Vec<Enum>, E> {
        match self.system() {
            System::Sqlite | System::LibSql | System::Mysql => Ok(Vec::new()),
            System::Postgres => postgres::enums(self.executor, &self.schema_name().await?).await,
        }
    }

    pub async fn inspect(&self) -> Result<Schema, E> {
        Ok(Schema {
            name: self.schema_name().await?,
            tables: dispatch!(self, tables(None))?,
            views: self.views().await?,
            enums: self.enums().await?,
        })
    }
}
