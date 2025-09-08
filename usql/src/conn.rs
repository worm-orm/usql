use alloc::boxed::Box;
use usql_core::{Connection, Connector, Executor, util::next};

use crate::{
    error::Error, query::IntoQuery, row::Row, stmt::Stmt, stream::QueryStream, target::Target,
    trans::Trans,
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

    pub fn as_target(&self) -> Target<'_, B> {
        Target::Conn(self)
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

    pub async fn fetch<'this, 'query, 'stream, Q>(
        &'this self,
        query: Q,
    ) -> Result<QueryStream<'stream, B>, Error<B>>
    where
        Q: IntoQuery<'query, B>,
        'query: 'stream,
        'this: 'stream,
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

    pub async fn fetch_one<'a, Q>(&self, query: Q) -> Result<Row<B>, Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        let mut stream = self.fetch(query).await?;
        match next(&mut stream).await {
            Some(ret) => ret,
            None => Err(Error::NotFound),
        }
    }

    pub async fn exec<'a, Q>(&self, query: Q) -> Result<(), Error<B>>
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

impl<B> Executor for Conn<B>
where
    B: Connector,
{
    type Connector = B;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        self.conn.db_info()
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
        self.conn.prepare(query)
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: alloc::vec::Vec<usql_value::ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        self.conn.query(stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: alloc::vec::Vec<usql_value::ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        self.conn.exec(stmt, params)
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        self.conn.exec_batch(stmt)
    }
}
