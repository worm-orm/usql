use std::string::String;

use super::{connector::Sqlite, error::Error};

pub struct Statement {
    pub(super) sql: String,
}

impl usql_core::Statement for Statement {
    type Connector = Sqlite;

    fn finalize(self) -> Result<(), Error> {
        Ok(())
    }
}
