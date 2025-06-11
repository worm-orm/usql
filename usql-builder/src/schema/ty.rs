use core::fmt::{self, Write};

use alloc::{borrow::Cow, string::ToString};
use usql::System;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnType<'a> {
    SmallInt,
    Int,
    BigInt,
    Char(u64),
    VarChar(u64),
    Text,
    Float,
    Double,
    Bool,
    Date,
    DateTime,
    Time,
    Binary,
    Uuid,
    Json,
    Other(Cow<'a, str>),
}

fn postgres(_kind: &ColumnType<'_>, _out: &mut dyn Write) -> fmt::Result {
    todo!()
}

fn sqlite(kind: &ColumnType<'_>, out: &mut dyn Write) -> fmt::Result {
    match kind {
        ColumnType::Bool | ColumnType::SmallInt | ColumnType::Int | ColumnType::BigInt => {
            out.write_str("INTEGER")
        }
        ColumnType::Text
        | ColumnType::Date
        | ColumnType::DateTime
        | ColumnType::Time
        | ColumnType::VarChar(_)
        | ColumnType::Char(_) => out.write_str("TEXT"),
        ColumnType::Binary => out.write_str("BLOB"),
        ColumnType::Float | ColumnType::Double => out.write_str("REAL"),
        ColumnType::Uuid => out.write_str("BLOB"),
        ColumnType::Json => out.write_str("TEXT"),
        _ => unreachable!(),
    }
}

fn mysql(_kind: &ColumnType<'_>, _out: &mut dyn Write) -> fmt::Result {
    todo!()
}

pub fn write_sql_type(
    kind: &ColumnType<'_>,
    out: &mut dyn fmt::Write,
    dialect: System,
) -> fmt::Result {
    match dialect {
        System::Postgres => postgres(kind, out),
        System::Sqlite | System::LibSql => sqlite(kind, out),
        System::Mysql => mysql(kind, out),
    }
}
