use core::ops::Deref;

use crate::{row::Row, stmt::Stmt};
use usql_core::{Connector, Executor, Transaction};
use usql_value::ValueCow;

use super::{LibSql, LibSqlInfo};

pub struct Trans(pub libsql::Transaction);

impl Transaction<'_> for Trans {
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

impl Executor for Trans {
    type Connector = LibSql;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        LibSqlInfo {}
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
        async move { Ok(Stmt(self.0.deref().prepare(query).await?)) }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: std::vec::Vec<ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        let stream = async_stream::try_stream! {
            let mut rows = stmt.0.query(params).await?;

            while let Some(next) = rows.next().await? {
                yield Row(next);
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
            stmt.0.execute(params).await?;
            Ok(())
        }
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            self.0.execute_batch(stmt).await?;
            Ok(())
        }
    }
}
