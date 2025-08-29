use usql_core::Executor;

use crate::connector::Postgres;

pub struct Transaction<'conn>(pub(crate) deadpool_postgres::Transaction<'conn>);

impl<'conn> usql_core::Transaction<'conn> for Transaction<'conn> {
    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as usql_core::Connector>::Error>> + Send
    {
        async move { todo!() }
    }

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as usql_core::Connector>::Error>> + Send
    {
        async move { todo!() }
    }
}

impl<'conn> Executor for Transaction<'conn> {
    type Connector = Postgres;

    fn db_info(&self) -> <Self::Connector as usql_core::Connector>::Info {
        todo!()
    }

    fn prepare<'a>(
        &'a self,
        query: &'a str,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as usql_core::Connector>::Statement,
            <Self::Connector as usql_core::Connector>::Error,
        >,
    > + Send
    + 'a {
        async move { todo!() }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as usql_core::Connector>::Statement,
        params: Vec<usql_value::ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        todo!()
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as usql_core::Connector>::Statement,
        params: Vec<usql_value::ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as usql_core::Connector>::Error>> + Send + 'a
    {
        async move { todo!() }
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as usql_core::Connector>::Error>> + Send + 'a
    {
        async move { todo!() }
    }
}
