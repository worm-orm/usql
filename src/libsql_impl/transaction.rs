use core::ops::Deref;

use crate::{Connection, Transaction};

use super::LibSql;

impl Transaction<'_> for libsql::Transaction {
    type Connector = LibSql;

    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send {
        async move { Ok(self.commit().await?) }
    }

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send {
        async move { Ok(self.rollback().await?) }
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
        async move { Ok(self.deref().prepare(query).await?) }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> crate::QueryStream<'a, Self::Connector> {
        <libsql::Connection as Connection>::query(self, stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: std::vec::Vec<crate::Value>,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send + 'a
    {
        <libsql::Connection as Connection>::exec(self, stmt, params)
    }
}
