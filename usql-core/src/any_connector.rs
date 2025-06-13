#[allow(unused_imports)]
use alloc::{boxed::Box, vec::Vec};
use core::task::{Poll, ready};

use futures_core::{Stream, stream::BoxStream};
use pin_project_lite::pin_project;

use crate::{ColumnIndex, Connector, Executor, QueryStream, Transaction};

#[cfg(feature = "libsql")]
use super::libsql_impl::{
    LibSql, LibSqlConnection, LibSqlError, LibSqlInfo, LibSqlOptions, LibSqlPool, LibSqlRow,
    LibSqlStatement, LibSqlTransaction,
};
#[cfg(feature = "postgres")]
use super::postgres_impl::*;
#[cfg(feature = "sqlite")]
use super::sqlite_impl::{
    Sqlite, SqliteConn, SqliteDatabaseInfo, SqliteError, SqliteOptions, SqlitePool, SqliteRow,
    SqliteStatement, SqliteTransaction,
};
use super::{Connection, DatabaseInfo, Pool, Row, Statement};

macro_rules! missing_db {
    () => {
        panic!("Enable at least one of the following features: sqlite, libsql")
    };
}

pub struct AnyConnector;

impl Connector for AnyConnector {
    type Connection = AnyConn;

    type Statement = AnyStatement;

    type Row = AnyRow;

    type Error = AnyError;

    type Info = AnyInfo;

    type Pool = AnyPool;

    type Options = AnyOptions;

    fn create_pool(
        options: Self::Options,
    ) -> impl Future<Output = Result<Self::Pool, Self::Error>> + Send {
        async move {
            #[allow(unreachable_patterns)]
            match options {
                #[cfg(feature = "sqlite")]
                AnyOptions::Sqlite(options) => Sqlite::create_pool(options)
                    .await
                    .map(AnyPool::Sqlite)
                    .map_err(AnyError::Sqlite),
                #[cfg(feature = "libsql")]
                AnyOptions::Libsql(options) => LibSql::create_pool(options)
                    .await
                    .map(AnyPool::Libsql)
                    .map_err(AnyError::LibSql),
                _ => missing_db!(),
            }
        }
    }
}

#[non_exhaustive]
pub enum AnyOptions {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteOptions),
    #[cfg(feature = "libsql")]
    Libsql(LibSqlOptions),
}

#[cfg(feature = "sqlite")]
impl From<SqliteOptions> for AnyOptions {
    fn from(value: SqliteOptions) -> Self {
        Self::Sqlite(value)
    }
}

#[cfg(feature = "libsql")]
impl From<LibSqlOptions> for AnyOptions {
    fn from(value: LibSqlOptions) -> Self {
        Self::Libsql(value)
    }
}

#[non_exhaustive]
pub enum AnyPool {
    #[cfg(feature = "sqlite")]
    Sqlite(SqlitePool),
    #[cfg(feature = "libsql")]
    Libsql(LibSqlPool),
}

impl Pool for AnyPool {
    type Connector = AnyConnector;
    fn get(
        &self,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as crate::Connector>::Connection,
            <Self::Connector as crate::Connector>::Error,
        >,
    > + Send
    + '_ {
        async move {
            #[allow(unreachable_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                AnyPool::Sqlite(pool) => pool
                    .get()
                    .await
                    .map(AnyConn::Sqlite)
                    .map_err(AnyError::Sqlite),
                #[cfg(feature = "libsql")]
                AnyPool::Libsql(pool) => pool
                    .get()
                    .await
                    .map(AnyConn::Libsql)
                    .map_err(AnyError::LibSql),
                _ => missing_db!(),
            }
        }
    }
}

#[non_exhaustive]
pub enum AnyInfo {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteDatabaseInfo),
    #[cfg(feature = "libsql")]
    Libsql(LibSqlInfo),
}

impl DatabaseInfo for AnyInfo {
    fn variant(&self) -> super::system::System {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            AnyInfo::Sqlite(_) => super::system::System::Sqlite,
            #[cfg(feature = "libsql")]
            AnyInfo::Libsql(_) => super::system::System::LibSql,
            _ => missing_db!(),
        }
    }
}

#[non_exhaustive]
pub enum AnyConn {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteConn),
    #[cfg(feature = "libsql")]
    Libsql(LibSqlConnection),
}

impl Connection for AnyConn {
    type Transaction<'conn> = AnyTransaction<'conn>;

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, <Self::Connector as Connector>::Error>> + Send
    {
        async move {
            #[allow(unreachable_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                Self::Sqlite(conn) => conn
                    .begin()
                    .await
                    .map(AnyTransaction::Sqlite)
                    .map_err(AnyError::Sqlite),
                #[cfg(feature = "libsql")]
                Self::Libsql(conn) => conn
                    .begin()
                    .await
                    .map(AnyTransaction::LibSql)
                    .map_err(AnyError::LibSql),
                _ => missing_db!(),
            }
        }
    }
}

impl Executor for AnyConn {
    type Connector = AnyConnector;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(info) => AnyInfo::Sqlite(info.db_info()),
            #[cfg(feature = "libsql")]
            Self::Libsql(info) => AnyInfo::Libsql(info.db_info()),
            _ => missing_db!(),
        }
    }

    #[allow(unused_variables)]
    fn prepare<'a>(
        &'a self,
        query: &'a str,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as Connector>::Statement,
            <Self::Connector as Connector>::Error,
        >,
    > + Send
    + 'a {
        async move {
            #[allow(unreachable_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                Self::Sqlite(conn) => conn
                    .prepare(query)
                    .await
                    .map(AnyStatement::Sqlite)
                    .map_err(AnyError::Sqlite),
                #[cfg(feature = "libsql")]
                Self::Libsql(conn) => conn
                    .prepare(query)
                    .await
                    .map(AnyStatement::LibSql)
                    .map_err(AnyError::LibSql),
                _ => missing_db!(),
            }
        }
    }

    #[allow(unused_variables)]
    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<crate::ValueCow<'a>>,
    ) -> QueryStream<'a, Self::Connector> {
        #[allow(unreachable_patterns, irrefutable_let_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(sqlite) => {
                let AnyStatement::Sqlite(stmt) = stmt else {
                    panic!("Statement mismatch")
                };
                Box::pin(AnyQueryStream::<Sqlite> {
                    stream: <SqliteConn as Executor>::query(sqlite, stmt, params),
                })
            }
            #[cfg(feature = "libsql")]
            Self::Libsql(libsql) => {
                let AnyStatement::LibSql(stmt) = stmt else {
                    panic!("Statement mismatch")
                };
                Box::pin(AnyQueryStream::<LibSql> {
                    stream: <LibSqlConnection as Executor>::query(libsql, stmt, params),
                })
            }
            _ => missing_db!(),
        }
    }

    #[allow(unused_variables)]
    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<crate::ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            #[allow(unreachable_patterns, irrefutable_let_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                Self::Sqlite(sqlite) => {
                    let AnyStatement::Sqlite(stmt) = stmt else {
                        panic!("Statement mismatch")
                    };
                    <SqliteConn as Executor>::exec(sqlite, stmt, params)
                        .await
                        .map_err(Into::into)
                }
                #[cfg(feature = "libsql")]
                Self::Libsql(libsql) => {
                    let AnyStatement::LibSql(stmt) = stmt else {
                        panic!("Statement mismatch")
                    };
                    <LibSqlConnection as Executor>::exec(libsql, stmt, params)
                        .await
                        .map_err(Into::into)
                }
                _ => missing_db!(),
            }
        }
    }
}

#[non_exhaustive]
pub enum AnyRow {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteRow),
    #[cfg(feature = "libsql")]
    Libsql(LibSqlRow),
}

#[allow(unused_variables, unreachable_patterns)]
impl Row for AnyRow {
    type Connector = AnyConnector;

    fn get<'a>(
        &'a self,
        index: ColumnIndex<'_>,
    ) -> Result<crate::ValueCow<'a>, <Self::Connector as Connector>::Error> {
        match self {
            #[cfg(feature = "sqlite")]
            AnyRow::Sqlite(row) => <SqliteRow as Row>::get(row, index).map_err(AnyError::Sqlite),
            #[cfg(feature = "libsql")]
            AnyRow::Libsql(row) => <LibSqlRow as Row>::get(row, index).map_err(AnyError::LibSql),
            _ => missing_db!(),
        }
    }

    fn get_typed<'a>(
        &'a self,
        index: ColumnIndex<'_>,
        ty: crate::Type,
    ) -> Result<crate::ValueCow<'a>, <Self::Connector as Connector>::Error> {
        match self {
            #[cfg(feature = "sqlite")]
            AnyRow::Sqlite(row) => {
                <SqliteRow as Row>::get_typed(row, index, ty).map_err(AnyError::Sqlite)
            }
            #[cfg(feature = "libsql")]
            AnyRow::Libsql(row) => {
                <LibSqlRow as Row>::get_typed(row, index, ty).map_err(AnyError::LibSql)
            }
            _ => missing_db!(),
        }
    }

    fn len(&self) -> usize {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            AnyRow::Sqlite(row) => row.len(),
            #[cfg(feature = "libsql")]
            AnyRow::Libsql(row) => row.len(),
            _ => missing_db!(),
        }
    }

    #[allow(unused_variables)]
    fn column_name(&self, idx: usize) -> Option<&str> {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            AnyRow::Sqlite(row) => row.column_name(idx),
            #[cfg(feature = "libsql")]
            AnyRow::Libsql(row) => <LibSqlRow as Row>::column_name(row, idx),
            _ => missing_db!(),
        }
    }
}

#[cfg(feature = "sqlite")]
impl From<SqliteRow> for AnyRow {
    fn from(value: SqliteRow) -> Self {
        Self::Sqlite(value)
    }
}

#[cfg(feature = "libsql")]
impl From<LibSqlRow> for AnyRow {
    fn from(value: LibSqlRow) -> Self {
        Self::Libsql(value)
    }
}

#[non_exhaustive]
pub enum AnyStatement {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteStatement),
    #[cfg(feature = "libsql")]
    LibSql(LibSqlStatement),
}

impl Statement for AnyStatement {
    type Connector = AnyConnector;

    fn finalize(self) -> Result<(), <Self::Connector as Connector>::Error> {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(stmt) => stmt.finalize().map_err(AnyError::Sqlite),
            #[cfg(feature = "libsql")]
            Self::Libsql(stmt) => stmt.finalize().map_err(AnyError::LibSql),
            _ => missing_db!(),
        }
    }
}

#[non_exhaustive]
pub enum AnyTransaction<'conn> {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteTransaction<'conn>),
    #[cfg(feature = "libsql")]
    LibSql(LibSqlTransaction),
    #[cfg(feature = "postgres")]
    Postgres(PgTrans<'conn>),
    #[cfg(all(not(feature = "sqlite"), not(feature = "postgres")))]
    Invariant(core::marker::PhantomData<&'conn ()>),
}

impl<'conn> Transaction<'conn> for AnyTransaction<'conn> {
    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        async move {
            #[allow(unreachable_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                Self::Sqlite(tx) => <SqliteTransaction as Transaction>::commit(tx)
                    .await
                    .map_err(Into::into),
                #[cfg(feature = "libsql")]
                Self::LibSql(tx) => <LibSqlTransaction as Transaction>::commit(tx)
                    .await
                    .map_err(Into::into),
                _ => missing_db!(),
            }
        }
    }

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        async move {
            #[allow(unreachable_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                Self::Sqlite(tx) => <SqliteTransaction as Transaction>::rollback(tx)
                    .await
                    .map_err(Into::into),
                #[cfg(feature = "libsql")]
                Self::LibSql(tx) => <LibSqlTransaction as Transaction>::rollback(tx)
                    .await
                    .map_err(Into::into),
                _ => missing_db!(),
            }
        }
    }
}

impl Executor for AnyTransaction<'_> {
    type Connector = AnyConnector;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(info) => AnyInfo::Sqlite(info.db_info()),
            #[cfg(feature = "libsql")]
            Self::LibSql(info) => AnyInfo::Libsql(info.db_info()),
            _ => missing_db!(),
        }
    }

    #[allow(unused_variables)]
    fn prepare<'a>(
        &'a self,
        query: &'a str,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as Connector>::Statement,
            <Self::Connector as Connector>::Error,
        >,
    > + Send
    + 'a {
        async move {
            #[allow(unreachable_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                Self::Sqlite(conn) => conn
                    .prepare(query)
                    .await
                    .map(AnyStatement::Sqlite)
                    .map_err(AnyError::Sqlite),
                #[cfg(feature = "libsql")]
                Self::LibSql(conn) => conn
                    .prepare(query)
                    .await
                    .map(AnyStatement::LibSql)
                    .map_err(AnyError::LibSql),
                _ => missing_db!(),
            }
        }
    }

    #[allow(unused_variables)]
    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<crate::ValueCow<'a>>,
    ) -> QueryStream<'a, Self::Connector> {
        #[allow(unreachable_patterns, irrefutable_let_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            Self::Sqlite(sqlite) => {
                let AnyStatement::Sqlite(stmt) = stmt else {
                    panic!("Statement mismatch")
                };
                Box::pin(AnyQueryStream::<Sqlite> {
                    stream: <SqliteTransaction as Executor>::query(sqlite, stmt, params),
                })
            }
            #[cfg(feature = "libsql")]
            Self::LibSql(libsql) => {
                let AnyStatement::LibSql(stmt) = stmt else {
                    panic!("Statement mismatch")
                };
                Box::pin(AnyQueryStream::<LibSql> {
                    stream: <LibSqlTransaction as Executor>::query(libsql, stmt, params),
                })
            }
            _ => missing_db!(),
        }
    }

    #[allow(unused_variables)]
    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<crate::ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        async move {
            #[allow(unreachable_patterns, irrefutable_let_patterns)]
            match self {
                #[cfg(feature = "sqlite")]
                Self::Sqlite(sqlite) => {
                    let AnyStatement::Sqlite(stmt) = stmt else {
                        panic!("Statement mismatch")
                    };
                    <SqliteTransaction as Executor>::exec(sqlite, stmt, params)
                        .await
                        .map_err(Into::into)
                }
                #[cfg(feature = "libsql")]
                Self::LibSql(libsql) => {
                    let AnyStatement::LibSql(stmt) = stmt else {
                        panic!("Statement mismatch")
                    };
                    <LibSqlTransaction as Executor>::exec(libsql, stmt, params)
                        .await
                        .map_err(Into::into)
                }
                _ => missing_db!(),
            }
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum AnyError {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteError),
    #[cfg(feature = "libsql")]
    LibSql(LibSqlError),
    Message(&'static str),
}

impl core::fmt::Display for AnyError {
    #[allow(unused_variables)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            AnyError::Sqlite(err) => write!(f, "{}", err),
            #[cfg(feature = "libsql")]
            AnyError::LibSql(err) => write!(f, "{}", err),
            AnyError::Message(msg) => msg.fmt(f),
            _ => missing_db!(),
        }
    }
}

impl core::error::Error for AnyError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "sqlite")]
            AnyError::Sqlite(err) => Some(err),
            #[cfg(feature = "libsql")]
            AnyError::LibSql(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(feature = "sqlite")]
impl From<SqliteError> for AnyError {
    fn from(value: SqliteError) -> Self {
        Self::Sqlite(value)
    }
}

#[cfg(feature = "libsql")]
impl From<LibSqlError> for AnyError {
    fn from(value: LibSqlError) -> Self {
        Self::LibSql(value)
    }
}

pin_project! {
    struct AnyQueryStream<'a, T: Connector> {
        #[pin]
        stream: BoxStream<'a, Result<T::Row, T::Error>>,
    }
}

impl<T> Stream for AnyQueryStream<'_, T>
where
    T: Connector,
    T::Error: Into<AnyError>,
    T::Row: Into<AnyRow>,
{
    type Item = Result<AnyRow, AnyError>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.stream.poll_next(cx)) {
            Some(Ok(ret)) => Poll::Ready(Some(Ok(ret.into()))),
            Some(Err(err)) => Poll::Ready(Some(Err(err.into()))),
            None => Poll::Ready(None),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
