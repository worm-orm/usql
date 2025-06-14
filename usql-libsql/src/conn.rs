use std::boxed::Box;

use usql_core::{Connection, Executor};

use super::{LibSqlInfo, connector::LibSql};

impl Connection for libsql::Connection {
    type Transaction<'conn> = libsql::Transaction;

    fn begin(
        &mut self,
    ) -> impl Future<
        Output = Result<Self::Transaction<'_>, <Self::Connector as crate::Connector>::Error>,
    > + Send {
        async move { Ok(self.transaction().await?) }
    }
}

impl Executor for libsql::Connection {
    type Connector = LibSql;

    fn db_info(&self) -> <Self::Connector as crate::Connector>::Info {
        LibSqlInfo {}
    }

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
        async move { Ok(self.prepare(query).await?) }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> crate::QueryStream<'a, Self::Connector> {
        let stream = async_stream::try_stream! {
            let mut rows = stmt.query(params).await?;

            while let Some(next) = rows.next().await? {
                yield next;
            }
        };

        Box::pin(stream)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send + 'a
    {
        async move {
            stmt.execute(params).await?;
            Ok(())
        }
    }
}
