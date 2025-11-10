mod conn;
mod connector;
mod error;
mod pool;
mod query_result;
mod row;
mod statement;
mod traits;
mod transaction;
mod util;
mod worker;

pub use self::{
    connector::*, error::Error as SqliteError, pool::ManagerOptions as SqliteOptions,
    pool::Pool as SqlitePool, pool::PooledConn as SqliteConn, row::Row as SqliteRow,
    statement::Statement as SqliteStatement, transaction::Transaction as SqliteTransaction,
};

#[cfg(feature = "vector")]
pub fn init_vector() {
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }
}

pub use rusqlite::OpenFlags;
