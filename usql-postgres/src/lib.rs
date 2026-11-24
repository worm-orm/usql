//! PostgreSQL driver for usql.
//!
//! Connections are driven by tasks on a [Tokio](https://tokio.rs) runtime, so the pool
//! must be used from within one.

mod conn;
mod connector;
mod convert;
mod error;
mod options;
mod pool;
mod row;
mod stmt;
mod transaction;

pub use self::{
    conn::Conn as PostgresConn,
    connector::{Info as PostgresDatabaseInfo, Postgres},
    error::Error as PostgresError,
    options::{Options as PostgresOptions, Tls},
    pool::Pool as PostgresPool,
    row::Row as PostgresRow,
    stmt::Statement as PostgresStatement,
    transaction::Transaction as PostgresTransaction,
};

pub use deadpool_postgres;
#[cfg(feature = "rustls")]
pub use rustls;
pub use tokio_postgres;
