use crate::connector::Mysql;

/// Only valid on the connection that prepared it
pub struct Statement(pub(crate) mysql_async::Statement);

impl usql_core::Statement for Statement {
    type Connector = Mysql;
    fn finalize(self) -> Result<(), <Self::Connector as usql_core::Connector>::Error> {
        Ok(())
    }
}
