use std::boxed::Box;
use std::marker::PhantomData;
use std::string::ToString;

use futures_channel::oneshot;

use super::{Sqlite, SqliteDatabaseInfo, SqliteStatement};
use super::{conn::QueryStream, query_result::QueryResult, row::Row, traits::Params};
use super::{error::Error, worker::TransRequest};
use usql_core::{Connector, Executor};
use usql_value::ValueCow;

pub struct Transaction<'conn> {
    pub(crate) invariant: PhantomData<&'conn ()>,
    pub(crate) channel: flume::Sender<TransRequest>,
}

impl Transaction<'_> {
    pub async fn commit(self) -> Result<(), rusqlite::Error> {
        let (sx, wait) = oneshot::channel();
        self.channel
            .send_async(TransRequest::Commit { returns: sx })
            .await
            .expect("send");

        wait.await.expect("wait")
    }

    pub async fn rollback(self) -> Result<(), rusqlite::Error> {
        let (sx, wait) = oneshot::channel();
        self.channel
            .send_async(TransRequest::Rollup { returns: sx })
            .await
            .expect("send");

        wait.await.expect("wait")
    }

    pub async fn query<P: Params>(
        &self,
        sql: impl ToString,
        values: P,
    ) -> Result<QueryStream, Error> {
        let values = values.into_params();
        let stmt = sql.to_string();
        let (returns, rx) = flume::bounded(1);

        self.channel
            .send_async(TransRequest::Fetch {
                stmt,
                values,
                returns,
            })
            .await
            .map_err(|_| Error::Channel)?;

        Ok(QueryStream {
            rx: rx.into_stream(),
        })
    }

    pub async fn query_one<P: Params>(&self, sql: impl ToString, values: P) -> Result<Row, Error> {
        let mut stream = self.query(sql, values).await?;

        let Some(first) = usql_core::util::next(&mut stream).await else {
            return Err(Error::NotFound);
        };

        first
    }

    pub async fn exec<P: Params>(
        &self,
        sql: impl ToString,
        values: P,
    ) -> Result<QueryResult, Error> {
        let (sx, rx) = oneshot::channel();
        self.channel
            .send_async(TransRequest::Exec {
                stmt: sql.to_string(),
                values: values.into_params(),
                returns: sx,
            })
            .await
            .map_err(|_| Error::Channel)?;

        Ok(rx.await.map_err(|_| Error::Channel)??)
    }

    pub async fn exec_batch(&self, sql: impl ToString) -> Result<(), Error> {
        let (sx, rx) = oneshot::channel();
        self.channel
            .send_async(TransRequest::ExecBatch {
                stmt: sql.to_string(),
                returns: sx,
            })
            .await
            .map_err(|_| Error::Channel)?;

        Ok(rx.await.map_err(|_| Error::Channel)??)
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        let (sx, _) = oneshot::channel();
        self.channel.send(TransRequest::Rollup { returns: sx }).ok();
    }
}

impl<'conn> usql_core::Transaction<'conn> for Transaction<'conn> {
    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        async move { Ok(self.commit().await?) }
    }

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        async move { Ok(self.rollback().await?) }
    }
}

impl Executor for Transaction<'_> {
    type Connector = Sqlite;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        SqliteDatabaseInfo
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
        async move {
            Ok(SqliteStatement {
                sql: query.to_string(),
            })
        }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: std::vec::Vec<ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        let stream = async_stream::try_stream! {
            let mut stream = self.query(&stmt.sql, params).await?;

            while let Some(next) = usql_core::util::next(&mut stream).await.transpose()? {
                yield next
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
            self.exec(&stmt.sql, params).await?;
            Ok(())
        }
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            self.exec_batch(stmt).await?;
            Ok(())
        }
    }
}
