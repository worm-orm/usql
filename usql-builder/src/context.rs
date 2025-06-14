use alloc::{
    fmt::{self, Write},
    string::String,
    vec::Vec,
};

use usql_core::System;
use usql_value::ValueCow;

use crate::{error::Error, sql::SqlStmt};

pub struct Context<'a> {
    system: System,
    bindings: Vec<ValueCow<'a>>,
    output: String,
}

impl<'a> Context<'a> {
    pub fn new(system: System) -> Context<'a> {
        Context {
            system,
            bindings: Default::default(),
            output: Default::default(),
        }
    }

    pub fn dialect(&self) -> System {
        self.system
    }
    pub fn push<S: Into<ValueCow<'a>>>(&mut self, value: S) -> Result<(), Error> {
        let value = value.into();

        if !value.as_ref().is_null() {
            self.bindings.push(value);
            match self.system {
                System::Mysql | System::LibSql | System::Sqlite => self.write_str("?"),
                System::Postgres => {
                    write!(self.output, "${}", self.output.len())
                }
            }?;
        } else {
            self.write_str("NULL")?;
        }

        Ok(())
    }

    pub fn push_identifier(&mut self, identifier: &str) -> Result<(), Error>
    where
        Self: Sized,
    {
        write_identifier(identifier, &self.system, &mut self.output)?;
        Ok(())
    }

    pub fn build(self) -> SqlStmt<'a> {
        SqlStmt::new(self.output, self.bindings)
    }
}

impl<'a> fmt::Display for Context<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.output.fmt(f)
    }
}

impl<'a> Write for Context<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.output.write_str(s)
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.output.write_char(c)
    }

    fn write_fmt(&mut self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        self.output.write_fmt(args)
    }
}

pub fn write_identifier<W>(ident: &str, dialect: &System, out: &mut W) -> fmt::Result
where
    W: Write + ?Sized,
{
    match dialect {
        &System::Mysql => {
            write!(out, "`{ident}`")
        }
        &System::Sqlite | &System::LibSql | &System::Postgres => {
            write!(out, "\"{ident}\"")
        }
    }
}

// pub fn write_identifier_escape<W>(dialect: &System, out: &mut W) -> fmt::Result
// where
//     W: Write + ?Sized,
// {
//     match dialect {
//         &System::Mysql => {
//             write!(out, "`")
//         }
//         &System::Sqlite | &System::LibSql | &System::Postgres => {
//             write!(out, "\"")
//         }
//     }
// }
