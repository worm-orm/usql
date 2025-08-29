use crate::{conn::Conn, connector::Postgres};

pub struct Pool(pub(crate) deadpool_postgres::Pool);

impl usql_core::Pool for Pool {
    type Connector = Postgres;

    fn get(
        &self,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as usql_core::Connector>::Connection,
            <Self::Connector as usql_core::Connector>::Error,
        >,
    > + Send
    + '_ {
        async move {
            let conn = self.0.get().await?;
            Ok(Conn(conn))
        }
    }
}
