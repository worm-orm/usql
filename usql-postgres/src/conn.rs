use usql_core::{Connection, Connector, Executor};

use crate::{connector::Info, row::Row, stmt::Statement, transaction::Transaction};

use super::connector::Postgres;
use futures::{TryStreamExt, pin_mut};

pub struct Conn(pub(crate) deadpool_postgres::Object);

impl Executor for Conn {
    type Connector = Postgres;

    fn db_info(&self) -> <Self::Connector as usql_core::Connector>::Info {
        Info
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
        async move { Ok(Statement(self.0.prepare_cached(query).await?)) }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<usql_value::ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        let stream = async_stream::try_stream! {
            let stream = self.0
            .query_raw(&stmt.0, params.into_iter().map(|m| m.to_owned())).await?;

            pin_mut!(stream);

            while let Some(next) = stream.try_next().await? {
                yield Row(next);
            }
        };

        Box::pin(stream)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<usql_value::ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            self.0
                .execute_raw(&stmt.0, params.into_iter().map(|m| m.to_owned()))
                .await?;
            Ok(())
        }
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            self.0.batch_execute(stmt).await?;
            Ok(())
        }
    }
}

impl Connection for Conn {
    type Transaction<'conn>
        = Transaction<'conn>
    where
        Self: 'conn;

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, <Self::Connector as Connector>::Error>> + Send
    {
        async move {
            let trans = self.0.transaction().await?;
            Ok(Transaction(trans))
        }
    }
}
