//! MySQL driver for usql.
//!
//! Connections are driven by [mysql_async](https://docs.rs/mysql_async), so the pool
//! must be used from within a [Tokio](https://tokio.rs) runtime.

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
    conn::Conn as MysqlConn,
    connector::{Info as MysqlDatabaseInfo, Mysql},
    error::Error as MysqlError,
    options::Options as MysqlOptions,
    pool::Pool as MysqlPool,
    row::Row as MysqlRow,
    stmt::Statement as MysqlStatement,
    transaction::Transaction as MysqlTransaction,
};

pub use mysql_async;
