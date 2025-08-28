use alloc::boxed::Box;
use usql_core::{Connection, Connector, Executor, util::next};

use crate::{
    error::Error, query::IntoQuery, row::Row, stmt::Stmt, stream::QueryStream, trans::Trans,
};

pub struct Conn<B>
where
    B: Connector,
{
    conn: B::Connection,
}

impl<B: Connector> Conn<B> {
    pub fn new(conn: B::Connection) -> Conn<B> {
        Conn { conn }
    }

    pub fn into_inner(self) -> B::Connection {
        self.conn
    }
}

impl<B> Conn<B>
where
    B: Connector,
    B::Error: core::error::Error + Send + Sync,
    B::Statement: 'static,
{
    pub async fn prepare(&self, sql: &str) -> Result<Stmt<B>, Error<B>> {
        let stmt = self.conn.prepare(sql).await.map_err(Error::connector)?;
        Ok(Stmt::new(stmt))
    }

    pub async fn begin<'a>(&'a mut self) -> Result<Trans<'a, B>, Error<B>> {
        let trans = self.conn.begin().await.map_err(Error::connector)?;
        Ok(Trans::new(trans))
    }

    pub async fn fetch<'a, Q>(&'a self, query: Q) -> Result<QueryStream<'a, B>, Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        let mut query = query.into_query(&self.conn).await?;

        let stream = async_stream::stream! {
          let mut stream = self.conn.query(query.stmt.as_mut()?, query.bindings);

          while let Some(row) = next(&mut stream).await {
            yield row.map(|row| Row {row}).map_err(Error::connector)
          }
        };

        Ok(QueryStream {
            stream: Box::pin(stream),
        })
    }

    pub async fn fetch_one<'a, Q>(&'a self, query: Q) -> Result<Row<B>, Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        let mut stream = self.fetch(query).await?;
        match next(&mut stream).await {
            Some(ret) => ret,
            None => Err(Error::NotFound),
        }
    }

    pub async fn exec<'a, Q>(&'a self, query: Q) -> Result<(), Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        let mut query = query.into_query(&self.conn).await?;

        self.conn
            .exec(query.stmt.as_mut()?, query.bindings)
            .await
            .map_err(Error::connector)?;

        Ok(())
    }
}
