use usql_core::{Connection, Connector};

use crate::{Conn, Error, IntoQuery, QueryStream, Row, Trans, stmt::Stmt};

pub enum Target<'a, T: Connector> {
    Conn(&'a Conn<T>),
    Trans(Trans<'a, T>),
}

impl<'c, B: Connector> Target<'c, B>
where
    B::Connection: 'c,
    <B::Connection as Connection>::Transaction<'c>: Send + Sync,
    B::Error: core::error::Error + Send + Sync,
    B::Statement: 'static,
{
    pub async fn prepare(&self, sql: &str) -> Result<Stmt<B>, Error<B>> {
        match self {
            Self::Conn(conn) => conn.prepare(sql).await,
            Self::Trans(trans) => trans.prepare(sql).await,
        }
    }

    pub async fn fetch<'a, 'b: 'a, Q>(&'b self, query: Q) -> Result<QueryStream<'a, B>, Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        match self {
            Self::Conn(conn) => conn.fetch(query).await,
            Self::Trans(trans) => trans.fetch(query).await,
        }
    }

    pub async fn fetch_one<'a, 'b: 'a, Q>(&'b self, query: Q) -> Result<Row<B>, Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        match self {
            Self::Conn(conn) => conn.fetch_one(query).await,
            Self::Trans(trans) => trans.fetch_one(query).await,
        }
    }

    pub async fn exec<'a, Q>(&'a self, query: Q) -> Result<(), Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        match self {
            Self::Conn(conn) => conn.exec(query).await,
            Self::Trans(trans) => trans.exec(query).await,
        }
    }

    pub async fn commit(self) -> Result<(), Error<B>> {
        match self {
            Self::Conn(_) => Ok(()),
            Self::Trans(trans) => trans.commit().await,
        }
    }

    pub async fn rollback(self) -> Result<(), Error<B>> {
        match self {
            Self::Conn(_) => Ok(()),
            Self::Trans(trans) => trans.rollback().await,
        }
    }
}

impl<'a, T: Connector> From<Trans<'a, T>> for Target<'a, T> {
    fn from(value: Trans<'a, T>) -> Self {
        Target::Trans(value)
    }
}

impl<'a, T: Connector> From<&'a Conn<T>> for Target<'a, T> {
    fn from(value: &'a Conn<T>) -> Self {
        Target::Conn(value)
    }
}
