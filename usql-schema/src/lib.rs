//! Reads metadata, such as tables and indexes, from a database.
//!
//! Not everything is available everywhere. SQLite only has the names of constraints, and
//! the expressions of indexes and generated columns, as part of an SQL text.
//! Check constraints are not read.

mod convert;
mod error;
mod inspector;
mod model;
mod postgres;
mod rows;
mod sqlite;

pub use self::{
    convert::{Conversion, Omitted},
    error::Error,
    inspector::Inspector,
    model::*,
};
