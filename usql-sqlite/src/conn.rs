use super::{
    SqliteDatabaseInfo, SqliteStatement,
    connector::Sqlite,
    error::Error,
    query_result::QueryResult,
    row::Row,
    traits::Params,
    transaction::Transaction,
    worker::{Request, open_worker},
};
use futures_channel::oneshot;
use futures_core::{Stream, ready};
use pin_project_lite::pin_project;
use rusqlite::OpenFlags;
use std::{any::Any, boxed::Box, marker::PhantomData, path::Path, string::ToString, task::Poll};
use usql_core::{Connection, Connector, Executor};
use usql_value::ValueCow;
pub struct Conn {
    channel: flume::Sender<Request>,
}

impl Conn {
    pub async fn open(path: impl AsRef<Path>, flags: OpenFlags) -> Result<Conn, Error> {
        let channel = open_worker(flags, Some(path.as_ref().to_path_buf())).await?;

        Ok(Conn { channel })
    }

    pub async fn open_memory(flags: OpenFlags) -> Result<Conn, Error> {
        let channel = open_worker(flags, None).await?;

        Ok(Conn { channel })
    }

    pub async fn begin_transaction(&mut self) -> Result<Transaction<'_>, Error> {
        let (sx, rx) = flume::bounded(1);
        let (ready_sx, ready_rx) = oneshot::channel();

        self.channel
            .send_async(Request::Begin {
                channel: rx,
                ready: ready_sx,
            })
            .await
            .map_err(|_| Error::Channel)?;

        ready_rx.await.map_err(|_| Error::Channel)??;

        Ok(Transaction {
            channel: sx,
            invariant: PhantomData,
        })
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
            .send_async(Request::Fetch {
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

        let Some(first) = stream.next().await? else {
            return Err(Error::NotFound);
        };

        Ok(first)
    }

    pub async fn exec<P: Params>(
        &self,
        sql: impl ToString,
        values: P,
    ) -> Result<QueryResult, Error> {
        let (sx, rx) = oneshot::channel();
        self.channel
            .send_async(Request::Exec {
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
            .send_async(Request::ExecBatch {
                stmt: sql.to_string(),
                returns: sx,
            })
            .await
            .map_err(|_| Error::Channel)?;

        Ok(rx.await.map_err(|_| Error::Channel)??)
    }

    #[allow(unused)]
    pub async fn with<F, R>(&self, func: F) -> Result<R, Error>
    where
        F: FnOnce(&rusqlite::Connection) -> Result<R, rusqlite::Error> + Send + 'static,
        R: Send + 'static,
    {
        let (sx, rx) = oneshot::channel();

        self.channel
            .send_async(Request::With {
                func: Box::new(|conn| func(conn).map(|e| Box::new(e) as Box<dyn Any + Send>)),
                returns: sx,
            })
            .await
            .map_err(|_| Error::Channel)?;

        let ret = rx.await.map_err(|_| Error::Channel)??;

        Ok(*ret.downcast().expect("downcast"))
    }
}

pin_project! {
    pub struct QueryStream {
        #[pin]
        pub(crate)rx: flume::r#async::RecvStream<'static,Result<Row, rusqlite::Error>>
    }
}

impl QueryStream {
    pub async fn next(&mut self) -> Result<Option<Row>, Error> {
        Ok(usql_core::util::next(&mut self.rx).await.transpose()?)
    }
}

impl Stream for QueryStream {
    type Item = Result<Row, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.rx.poll_next(cx)) {
            Some(Ok(ret)) => Poll::Ready(Some(Ok(ret))),
            Some(Err(err)) => Poll::Ready(Some(Err(Error::Sqlite(err)))),
            None => Poll::Ready(None),
        }
    }
}

impl Connection for Conn {
    type Transaction<'conn> = Transaction<'conn>;

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, <Self::Connector as Connector>::Error>> + Send
    {
        async move { self.begin_transaction().await }
    }
}

impl Executor for Conn {
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
