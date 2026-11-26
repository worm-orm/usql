//! Reads metadata, such as tables and indexes, from a database.
//!
//! Not everything is available everywhere. SQLite only has the names of constraints, and
//! the expressions of indexes and generated columns, as part of an SQL text. On MySQL a
//! schema is a database, and unique constraints are read as unique indexes.
//! Check constraints are not read.

mod convert;
mod error;
mod inspector;
mod model;
mod mysql;
mod postgres;
mod rows;
mod sqlite;

pub use self::{
    convert::{Conversion, Omitted},
    error::Error,
    inspector::Inspector,
    model::*,
};
