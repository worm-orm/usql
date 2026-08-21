use core::fmt::{self, Write};

use alloc::borrow::Cow;
use usql_core::System;

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
    Blob,
    Uuid,
    Json,
    Other(Cow<'a, str>),
}

fn postgres(kind: &ColumnType<'_>, _out: &mut dyn Write) -> fmt::Result {
    match kind {
        ColumnType::Bool => _out.write_str("BOOLEAN"),
        ColumnType::SmallInt => _out.write_str("SMALLINT"),
        ColumnType::Int => _out.write_str("INTEGER"),
        ColumnType::BigInt => _out.write_str("BIGINT"),
        ColumnType::Char(size) => write!(_out, "CHAR({size})"),
        ColumnType::VarChar(size) => write!(_out, "VARCHAR({size})"),
        ColumnType::Text => _out.write_str("TEXT"),
        ColumnType::Float => _out.write_str("REAL"),
        ColumnType::Double => _out.write_str("DOUBLE PRECISION"),
        ColumnType::Date => _out.write_str("DATE"),
        ColumnType::DateTime => _out.write_str("TIMESTAMP"),
        ColumnType::Time => _out.write_str("TIME"),
        ColumnType::Blob => _out.write_str("BYTEA"),
        ColumnType::Uuid => _out.write_str("UUID"),
        ColumnType::Json => _out.write_str("JSONB"),
        ColumnType::Other(other) => _out.write_str(other),
    }
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
        ColumnType::Blob => out.write_str("BLOB"),
        ColumnType::Float | ColumnType::Double => out.write_str("REAL"),
        ColumnType::Uuid => out.write_str("BLOB"),
        ColumnType::Json => out.write_str("TEXT"),
        ColumnType::Other(other) => out.write_str(other),
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
