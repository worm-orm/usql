use usql::{
    QueryStream, Trans,
    core::{Connection, Connector, Executor},
    value::ValueCow,
};

pub struct Exec<'a, B: Connector>
where
    B::Connection: 'a,
{
    pub(crate) conn: Trans<'a, B>,
}

impl<'a, B> Exec<'a, B>
where
    B: Connector,
    B::Connection: 'a,
{
    pub fn new(conn: Trans<'a, B>) -> Exec<'a, B> {
        Exec { conn }
    }
}

impl<'b, B> Executor for Exec<'b, B>
where
    B: Connector,
    <B::Connection as Connection>::Transaction<'b>: Send + Sync,
{
    type Connector = B;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        self.conn.db_info()
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
        async move { self.conn.prepare(query).await }
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> usql::core::QueryStream<'a, Self::Connector> {
        self.conn.query(stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move { self.conn.exec(stmt, params).await }
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move { self.conn.exec_batch(stmt).await }
    }
}
