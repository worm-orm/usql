use futures::{TryStreamExt, pin_mut};
use usql_core::{Connector, Executor};

use crate::{
    connector::{Info, Postgres},
    row::Row,
    stmt::Statement,
};

/// Rolled back when dropped without a commit
pub struct Transaction<'conn>(pub(crate) deadpool_postgres::Transaction<'conn>);

impl<'conn> usql_core::Transaction<'conn> for Transaction<'conn> {
    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        async move { Ok(self.0.commit().await?) }
    }

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        async move { Ok(self.0.rollback().await?) }
    }
}

impl Executor for Transaction<'_> {
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
