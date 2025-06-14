use async_lock::Mutex;
use libsql::Connection;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use usql_core::{Connector, Executor, QueryStream, ValueCow};

use crate::{row::Row, stmt::Stmt, transaction::Trans};

use super::{LibSqlInfo, connector::LibSql};

// pub type PooledConn = deadpool::managed::Object<Manager>;

#[derive(Debug)]
enum Source {
    Memory(Uuid),
    Path(PathBuf),
}

pub struct ManagerOptions {
    pub path: Option<PathBuf>,
    pub flags: libsql::OpenFlags,
}

pub struct Manager {
    source: Source,
    flags: libsql::OpenFlags,
    _persist: Mutex<Option<libsql::Connection>>,
}

impl Manager {
    pub fn file(path: impl AsRef<Path>) -> Manager {
        Manager::new(ManagerOptions {
            path: Some(path.as_ref().to_path_buf()),
            flags: libsql::OpenFlags::default(),
        })
    }

    pub fn memory() -> Manager {
        Manager::new(ManagerOptions {
            path: None,
            flags: libsql::OpenFlags::default(),
        })
    }

    pub fn new(options: ManagerOptions) -> Manager {
        let source = match options.path {
            Some(path) => Source::Path(path),
            None => Source::Memory(Uuid::new_v4()),
        };

        Manager {
            source,
            flags: options.flags,
            _persist: Mutex::new(None),
        }
    }
}

impl deadpool::managed::Manager for Manager {
    type Type = Connection;

    type Error = super::error::Error;

    fn create(&self) -> impl Future<Output = Result<Self::Type, Self::Error>> + Send {
        async move {
            match &self.source {
                Source::Path(path) => {
                    //
                    let db = libsql::Builder::new_local(path)
                        .flags(self.flags)
                        .build()
                        .await?;
                    Ok(db.connect()?)
                }
                Source::Memory(id) => {
                    // let connection =
                    //     || Conn::open(format!("file:{}?mode=memory&cache=shared", id), self.flags);

                    let connection = || async move {
                        let db = libsql::Builder::new_local(format!(
                            "file:{}?mode=memory&cache=shared",
                            id
                        ))
                        .flags(self.flags)
                        .build()
                        .await?;
                        db.connect()
                    };

                    {
                        let mut persist = self._persist.lock().await;
                        if persist.is_none() {
                            *persist = Some(connection().await?);
                        }
                    }

                    Ok(connection().await?)
                }
            }
        }
    }

    fn recycle(
        &self,
        _obj: &mut Self::Type,
        _metrics: &deadpool::managed::Metrics,
    ) -> impl Future<Output = deadpool::managed::RecycleResult<Self::Error>> + Send {
        async move { Ok(()) }
    }
}

#[derive(Clone)]
pub struct Pool(deadpool::managed::Pool<Manager>);

impl Pool {
    pub fn new(manager: Manager) -> Pool {
        Pool(deadpool::managed::Pool::builder(manager).build().unwrap())
    }
}

impl usql_core::Pool for Pool {
    type Connector = LibSql;

    fn get(
        &self,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as Connector>::Connection,
            <Self::Connector as Connector>::Error,
        >,
    > + Send
    + '_ {
        async move { Ok(Conn(self.0.get().await?)) }
    }
}

pub struct Conn(deadpool::managed::Object<Manager>);

impl usql_core::Connection for Conn {
    type Transaction<'conn> = Trans;

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, <Self::Connector as Connector>::Error>> + Send
    {
        async move {
            let conn = self.0.transaction().await?;
            Ok(Trans(conn))
        }
    }
}

impl Executor for Conn {
    type Connector = LibSql;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        LibSqlInfo
    }

    fn prepare<'a>(
        &'a self,
        query: &'a str,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as Connector>::Statement,
            <Self::Connector as Connector>::Error,
        >,
    > + Send
    + 'a {
        async move { Ok(Stmt(self.0.prepare(query).await?)) }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: std::vec::Vec<ValueCow<'a>>,
    ) -> QueryStream<'a, Self::Connector> {
        let stream = async_stream::try_stream! {
            let mut rows = stmt.0.query(params).await?;

            while let Some(next) = rows.next().await? {
                yield Row(next);
            }
        };

        Box::pin(stream)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: std::vec::Vec<ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            stmt.0.execute(params).await?;
            Ok(())
        }
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            self.0.execute_batch(stmt).await?;
            Ok(())
        }
    }
}
