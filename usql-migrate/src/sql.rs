use std::collections::HashMap;

use tokio::fs;
use usql_core::{Connection, Connector, DatabaseInfo, Executor, System};

use crate::{Exec, MigrationLoader, Runner};

#[derive(Default)]
pub struct SqlLoader;

const EXTS: &'static [&'static str] = &["sql", "sqlite", "libsql", "postgres"];

impl<B> MigrationLoader<B> for SqlLoader
where
    B: Connector,
    for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
{
    type Migration = SqlRunner;

    type Error = B::Error;

    fn can_load<'a>(&'a self, path: &'a std::path::Path) -> impl Future<Output = bool> + Send + 'a {
        async move {
            if !path.is_dir() {
                return false;
            }

            for ext in EXTS {
                let up = path.join(format!("up.{ext}"));
                if fs::metadata(&up).await.is_ok() {
                    return true;
                }
            }

            false
        }
    }

    fn load<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> impl Future<Output = Result<Self::Migration, Self::Error>> + Send + 'a {
        async move {
            let mut up = Script::default();
            let mut down = Script::default();
            for ext in EXTS {
                let up_path = path.join(format!("up.{ext}"));
                let down_path = path.join(format!("down.{ext}"));

                if fs::metadata(&up_path).await.is_ok() {
                    let content = fs::read_to_string(&up_path).await.unwrap();
                    match *ext {
                        "postgres" => {
                            up.scripts.insert(System::Postgres, content);
                        }
                        "sqlite" => {
                            up.scripts.insert(System::Sqlite, content);
                        }
                        "libsql" => {
                            up.scripts.insert(System::LibSql, content);
                        }
                        _ => {
                            up.default = Some(content);
                        }
                    };
                }

                if fs::metadata(&down_path).await.is_ok() {
                    let content = fs::read_to_string(&down_path).await.unwrap();
                    match *ext {
                        "postgres" => {
                            down.scripts.insert(System::Postgres, content);
                        }
                        "sqlite" => {
                            down.scripts.insert(System::Sqlite, content);
                        }
                        "libsql" => {
                            down.scripts.insert(System::LibSql, content);
                        }
                        _ => {
                            down.default = Some(content);
                        }
                    };
                }
            }

            Ok(SqlRunner { up, down })
        }
    }
}

#[derive(Debug, Default)]
struct Script {
    scripts: HashMap<System, String>,
    default: Option<String>,
}

impl Script {
    async fn run<B: Connector>(&self, exec: &Exec<'_, B>) -> Result<(), B::Error>
    where
        for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
    {
        let script = if let Some(script) = &self.scripts.get(&exec.conn.db_info().variant()) {
            script
        } else {
            match &self.default {
                Some(ret) => ret,
                None => {
                    panic!("no default script")
                }
            }
        };

        exec.exec_batch(script).await
    }
}

#[derive(Debug)]
pub struct SqlRunner {
    up: Script,
    down: Script,
}

impl<B: Connector> Runner<B> for SqlRunner
where
    for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
{
    type Error = B::Error;

    fn up<'a>(
        &'a self,
        executor: &'a crate::Exec<'_, B>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'a {
        async move {
            self.up.run(executor).await?;
            Ok(())
        }
    }

    fn down<'a>(
        &'a self,
        executor: &'a crate::Exec<'_, B>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'a {
        async move {
            self.down.run(executor).await?;
            Ok(())
        }
    }
}
