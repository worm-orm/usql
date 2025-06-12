use async_lock::Mutex;
use libsql::Connection;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{Executor, Pool};

use super::{LibSqlInfo, connector::LibSql};

pub type PooledConn = deadpool::managed::Object<Manager>;

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

    fn create(&self) -> impl futures_core::Future<Output = Result<Self::Type, Self::Error>> + Send {
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
                        let db = libsql::Builder::new_local(alloc::format!(
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
    ) -> impl futures_core::Future<Output = deadpool::managed::RecycleResult<Self::Error>> + Send
    {
        async move { Ok(()) }
    }
}

#[derive(Clone)]
pub struct LibSqlPool(deadpool::managed::Pool<Manager>);

impl LibSqlPool {
    pub fn new(manager: Manager) -> LibSqlPool {
        LibSqlPool(deadpool::managed::Pool::builder(manager).build().unwrap())
    }

    // pub async fn get(
    //     &self,
    // ) -> Result<deadpool::managed::Object<Manager>, PoolError<libsql::Error>> {
    //     self.0.get().await
    // }
}

impl Pool for LibSqlPool {
    type Connector = LibSql;

    fn get(
        &self,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as crate::Connector>::Connection,
            <Self::Connector as crate::Connector>::Error,
        >,
    > + Send
    + '_ {
        async move { Ok(self.0.get().await?) }
    }
}

impl crate::Connection for PooledConn {
    type Transaction<'conn> = libsql::Transaction;

    fn begin(
        &mut self,
    ) -> impl Future<
        Output = Result<Self::Transaction<'_>, <Self::Connector as crate::Connector>::Error>,
    > + Send {
        async move { <libsql::Connection as crate::Connection>::begin(self.as_mut()).await }
    }
}

impl Executor for PooledConn {
    type Connector = LibSql;

    fn db_info(&self) -> <Self::Connector as crate::Connector>::Info {
        LibSqlInfo
    }

    fn prepare<'a>(
        &'a self,
        query: &'a str,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as crate::Connector>::Statement,
            <Self::Connector as crate::Connector>::Error,
        >,
    > + Send
    + 'a {
        async move { Ok(self.as_ref().prepare(query).await?) }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> crate::QueryStream<'a, Self::Connector> {
        <libsql::Connection as crate::Executor>::query(self.as_ref(), stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send + 'a
    {
        <libsql::Connection as crate::Executor>::exec(self.as_ref(), stmt, params)
    }
}
