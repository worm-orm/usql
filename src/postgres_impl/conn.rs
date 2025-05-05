use crate::Connection;

use super::connector::Postgres;

impl Connection for deadpool_postgres::Object {
    type Connector = Postgres;

    type Transaction<'conn>
        = tokio_postgres::Transaction<'conn>
    where
        Self: 'conn;

    fn db_info(&self) -> <Self::Connector as crate::Connector>::Info {
        todo!()
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
        async move { self.prepare_cached(query).await }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: alloc::vec::Vec<crate::Value>,
    ) -> crate::QueryStream<'a, Self::Connector> {
        todo!()
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as crate::Connector>::Statement,
        params: alloc::vec::Vec<crate::Value>,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send + 'a
    {
        async move { todo!() }
    }

    fn begin(
        &mut self,
    ) -> impl Future<
        Output = Result<Self::Transaction<'_>, <Self::Connector as crate::Connector>::Error>,
    > + Send {
        async move { todo!() }
    }
}
