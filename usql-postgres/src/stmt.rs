use crate::connector::Postgres;

pub struct Statement(pub(crate) tokio_postgres::Statement);

impl usql_core::Statement for Statement {
    type Connector = Postgres;
    fn finalize(self) -> Result<(), <Self::Connector as usql_core::Connector>::Error> {
        Ok(())
    }
}
