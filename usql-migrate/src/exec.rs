use usql_core::{Connection, Connector, Executor};

pub struct Exec<'a, B: Connector>
where
    B::Connection: 'a,
{
    pub(crate) conn: <B::Connection as Connection>::Transaction<'a>,
}

impl<'a, B> Exec<'a, B>
where
    B: Connector,
    B::Connection: 'a,
{
    pub fn new(conn: <B::Connection as Connection>::Transaction<'a>) -> Exec<'a, B> {
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
        todo!()
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
        params: Vec<usql_core::ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        self.conn.query(stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<usql_core::ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move { self.conn.exec(stmt, params).await }
    }
}
