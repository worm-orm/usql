use std::{
    format,
    path::{Path, PathBuf},
};

use async_lock::Mutex;
use uuid::Uuid;

use crate::{Connection, Executor};

use super::{
    conn::Conn,
    connector::{Sqlite, SqliteDatabaseInfo},
    error::Error,
    transaction::Transaction,
};

pub type PooledConn = deadpool::managed::Object<Manager>;

#[derive(Debug)]
enum Source {
    Memory(Uuid),
    Path(PathBuf),
}

pub struct ManagerOptions {
    pub path: Option<PathBuf>,
    pub flags: rusqlite::OpenFlags,
}

pub struct Manager {
    source: Source,
    flags: rusqlite::OpenFlags,
    _persist: Mutex<Option<Conn>>,
}

impl Manager {
    pub fn file(path: impl AsRef<Path>) -> Manager {
        Manager::new(ManagerOptions {
            path: Some(path.as_ref().to_path_buf()),
            flags: rusqlite::OpenFlags::default(),
        })
    }

    pub fn memory() -> Manager {
        Manager::new(ManagerOptions {
            path: None,
            flags: rusqlite::OpenFlags::default(),
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
    type Type = Conn;

    type Error = Error;

    fn create(&self) -> impl futures_core::Future<Output = Result<Self::Type, Self::Error>> + Send {
        async move {
            match &self.source {
                Source::Path(path) => Conn::open(path, self.flags).await,
                Source::Memory(id) => {
                    let connection =
                        || Conn::open(format!("file:{}?mode=memory&cache=shared", id), self.flags);

                    {
                        let mut persist = self._persist.lock().await;
                        if persist.is_none() {
                            *persist = Some(connection().await?);
                        }
                    }

                    connection().await
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
pub struct Pool(deadpool::managed::Pool<Manager>);

impl Pool {
    pub fn new(manager: Manager) -> Pool {
        Pool(deadpool::managed::Pool::builder(manager).build().unwrap())
    }

    // pub async fn get(&self) -> Result<deadpool::managed::Object<Manager>, Error> {
    //     Ok(self.0.get().await?)
    // }
}

impl crate::Pool for Pool {
    type Connector = Sqlite;

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

impl Connection for PooledConn {
    type Transaction<'conn> = Transaction<'conn>;

    fn db_info(&self) -> <Self::Connector as crate::Connector>::Info {
        SqliteDatabaseInfo
    }

    fn begin(
        &mut self,
    ) -> impl Future<
        Output = Result<Self::Transaction<'_>, <Self::Connector as crate::Connector>::Error>,
    > + Send {
        async move { <Conn as Connection>::begin(self.as_mut()).await }
    }
}

impl Executor for PooledConn {
    type Connector = Sqlite;

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
        self.as_ref().prepare(query)
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> crate::QueryStream<'a, Self::Connector> {
        <Conn as Executor>::query(self.as_ref(), stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send + 'a
    {
        <Conn as Executor>::exec(self.as_ref(), stmt, params)
    }
}
