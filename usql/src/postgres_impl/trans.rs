use crate::Transaction;

use super::connector::Postgres;

impl<'conn> Transaction<'conn> for tokio_postgres::Transaction<'conn> {
    type Connector = Postgres;

    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send {
        async move { todo!() }
    }

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as crate::Connector>::Error>> + Send {
        async move { todo!() }
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
        async move { todo!() }
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
}
