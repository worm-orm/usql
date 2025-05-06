use std::string::String;

use super::connector::Sqlite;

pub struct Statement {
    pub(super) sql: String,
}

impl crate::Statement for Statement {
    type Connector = Sqlite;
}
