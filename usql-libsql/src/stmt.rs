use usql_core::Statement;

use crate::LibSql;

pub struct Stmt(pub libsql::Statement);

impl Statement for Stmt {
    type Connector = LibSql;

    fn finalize(mut self) -> Result<(), <Self::Connector as usql_core::Connector>::Error> {
        self.0.finalize();
        Ok(())
    }
}
