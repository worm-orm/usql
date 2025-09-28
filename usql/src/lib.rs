#![no_std]

extern crate alloc;

mod conn;
mod error;
mod from_row;
mod pool;
mod query;
mod row;
mod stmt;
mod stream;
mod target;
mod trans;
mod typed;

pub use usql_builder as builder;
pub use usql_core as core;
pub use usql_value as value;

pub use self::{
    conn::Conn,
    error::Error,
    from_row::FromRow,
    pool::Pool,
    query::IntoQuery,
    row::Row,
    stream::{FromRowStream, QueryStream},
    target::Target,
    trans::Trans,
    typed::Typed,
};

#[cfg(feature = "derive")]
pub use usql_macros::*;
