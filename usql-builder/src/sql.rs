use alloc::{
    fmt,
    string::{String, ToString},
    vec::Vec,
};
use usql::{System, ValueCow};

use crate::{
    Context, Error,
    statement::{self, Statement},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SqlStmt<'a> {
    pub sql: String,
    pub bindings: Vec<ValueCow<'a>>,
}

impl<'a> SqlStmt<'a> {
    pub fn new(sql: String, bindings: Vec<ValueCow<'a>>) -> SqlStmt<'a> {
        SqlStmt { sql, bindings }
    }
}

#[cfg(not(feature = "std"))]
impl<'a> fmt::Display for SqlStmt<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.sql)
    }
}

#[cfg(feature = "std")]
impl<'a> fmt::Display for SqlStmt<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let values = self
            .bindings
            .iter()
            .map(|m| m.as_ref().to_string())
            .collect::<Vec<_>>();

        let sql = sqlformat::format(
            &self.sql,
            &sqlformat::QueryParams::Indexed(values),
            &sqlformat::FormatOptions {
                indent: sqlformat::Indent::Spaces(2),
                uppercase: Some(true),
                lines_between_queries: 1,
                ignore_case_convert: None,
            },
        );

        write!(f, "{sql}")?;

        Ok(())
    }
}
