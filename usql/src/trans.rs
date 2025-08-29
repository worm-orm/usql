use alloc::boxed::Box;
use usql_core::{Connection, Connector, Executor, Transaction, util::next};

use crate::{Error, IntoQuery, QueryStream, Row, query, stmt::Stmt};

pub struct Trans<'a, B: Connector>
where
    B::Connection: 'a,
{
    trans: <B::Connection as Connection>::Transaction<'a>,
}

impl<'a, B: Connector> Trans<'a, B> {
    pub fn new(trans: <B::Connection as Connection>::Transaction<'a>) -> Trans<'a, B> {
        Trans { trans }
    }
}

impl<'c, B: Connector> Trans<'c, B>
where
    B::Connection: 'c,
    <B::Connection as Connection>::Transaction<'c>: Send + Sync,
    B::Error: core::error::Error + Send + Sync,
    B::Statement: 'static,
{
    pub async fn prepare(&self, sql: &str) -> Result<Stmt<B>, Error<B>> {
        let stmt = self.trans.prepare(sql).await.map_err(Error::connector)?;
        Ok(Stmt::new(stmt))
    }

    pub async fn fetch<'a, Q>(&'a self, query: Q) -> Result<QueryStream<'a, B>, Error<B>>
    where
        Q: IntoQuery<'a, B>,
    {
        let mut query = query.into_query(&self.trans).await?;

        let stream = async_stream::stream! {
          let mut stream = self.trans.query(query.stmt.as_mut()?, query.bindings);

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
        let mut query = query.into_query(&self.trans).await?;

        self.trans
            .exec(query.stmt.as_mut()?, query.bindings)
            .await
            .map_err(Error::connector)?;

        Ok(())
    }

    pub async fn commit(self) -> Result<(), Error<B>> {
        self.trans.commit().await.map_err(Error::connector)
    }

    pub async fn rollback(self) -> Result<(), Error<B>> {
        self.trans.rollback().await.map_err(Error::connector)
    }
}

impl<'c, B> Executor for Trans<'c, B>
where
    B: Connector,
{
    type Connector = B;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        self.trans.db_info()
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
        self.trans.prepare(query)
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: alloc::vec::Vec<usql_value::ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        self.trans.query(stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: alloc::vec::Vec<usql_value::ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        self.trans.exec(stmt, params)
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        self.trans.exec_batch(stmt)
    }
}
