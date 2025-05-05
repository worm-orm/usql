use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{AnyConnector, AnyError, AnyOptions, AnyPool, Connector};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workers: Option<usize>,
    #[serde(flatten)]
    pub kind: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DatabaseConfig {
    Sqlite(SqliteConfig),
    LibSql(LibSqlConfig),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SqliteConfig {
    Memory,
    Path(PathBuf),
}

#[cfg(feature = "sqlite")]
impl From<SqliteConfig> for AnyOptions {
    fn from(value: SqliteConfig) -> Self {
        use crate::SqliteOptions;
        let opts = match value {
            SqliteConfig::Memory => SqliteOptions {
                path: None,
                flags: Default::default(),
            },
            SqliteConfig::Path(path) => SqliteOptions {
                path: Some(path),
                flags: Default::default(),
            },
        };

        opts.into()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LibSqlConfig {
    Memory,
    Path(PathBuf),
}

#[cfg(feature = "libsql")]
impl From<LibSqlConfig> for AnyOptions {
    fn from(value: LibSqlConfig) -> Self {
        use crate::LibSqlOptions;
        let opts = match value {
            LibSqlConfig::Memory => LibSqlOptions {
                path: None,
                flags: Default::default(),
            },
            LibSqlConfig::Path(path) => LibSqlOptions {
                path: Some(path),
                flags: Default::default(),
            },
        };

        opts.into()
    }
}

impl Config {
    pub async fn crate_pool(self) -> Result<AnyPool, AnyError> {
        match self.kind {
            DatabaseConfig::Sqlite(sqlite_config) => {
                #[cfg(feature = "sqlite")]
                let pool = AnyConnector::create_pool(sqlite_config.into()).await;
                #[cfg(not(feature = "sqlite"))]
                let pool = Err(AnyError::Message("Sqlite feature not enabled"));
                pool
            }
            DatabaseConfig::LibSql(lib_sql_config) => {
                #[cfg(feature = "libsql")]
                let pool = AnyConnector::create_pool(lib_sql_config.into()).await;
                #[cfg(not(feature = "libsql"))]
                let pool = Err(AnyError::Message("Sqlite feature not enabled"));
                pool
            }
        }
    }
}
