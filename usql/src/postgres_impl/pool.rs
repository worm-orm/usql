use crate::Pool;

use super::connector::Postgres;

impl Pool for deadpool_postgres::Pool {
    type Connector = Postgres;

    fn get(
        &self,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as crate::Connector>::Connection,
            <Self::Connector as crate::Connector>::Error,
        >,
    > + Send
    + '_ {
        async move {
            let conn = self.get().await.unwrap();
            Ok(conn)
        }
    }
}
