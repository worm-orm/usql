use crate::{conn::Conn, connector::Mysql};

#[derive(Clone)]
pub struct Pool(pub(crate) mysql_async::Pool);

impl usql_core::Pool for Pool {
    type Connector = Mysql;

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
            let conn = self.0.get_conn().await?;
            Ok(Conn::new(conn))
        }
    }
}
