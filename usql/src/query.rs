use alloc::vec::Vec;
use usql_builder::{
    SqlStmt, StatementExt,
    mutate::{Insert, InsertReturning},
    schema::CreateTable,
    select::{QueryStmt, Selection},
};
use usql_core::{Connector, DatabaseInfo, Executor, Statement, ValueCow};

use crate::{error::Error, stmt::Stmt};

pub(crate) enum StmtRef<'a, B: Connector> {
    Borrow(&'a mut B::Statement),
    Owned(Option<B::Statement>),
}

impl<'a, B: Connector> StmtRef<'a, B> {
    pub fn as_mut(&mut self) -> Result<&mut B::Statement, Error<B>> {
        match self {
            StmtRef::Borrow(stmt) => Ok(stmt),
            StmtRef::Owned(stmt) => stmt
                .as_mut()
                .ok_or_else(|| Error::query("Statement already used")),
        }
    }
}

impl<'a, B: Connector> Drop for StmtRef<'a, B> {
    fn drop(&mut self) {
        match self {
            Self::Owned(stmt) => {
                if let Some(stmt) = stmt.take() {
                    stmt.finalize().ok();
                }
            }
            _ => {}
        }
    }
}

pub struct Query<'a, B: Connector> {
    pub(crate) stmt: StmtRef<'a, B>,
    pub(crate) bindings: Vec<ValueCow<'a>>,
}

pub trait IntoQuery<'a, B: Connector>
where
    B::Statement: 'static,
{
    fn into_query<E>(
        self,
        executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync;
}

impl<'a, B: Connector> IntoQuery<'a, B> for Query<'a, B>
where
    B::Statement: 'static,
{
    fn into_query<E>(
        self,
        _executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move { Ok(self) }
    }
}

impl<'a, B: Connector> IntoQuery<'a, B> for SqlStmt<'a>
where
    B::Statement: 'static,
    B::Error: core::error::Error,
{
    fn into_query<E>(
        self,
        executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Sync + Send,
    {
        async move {
            let stmt = executor.prepare(&self.sql).await.unwrap();
            Ok(Query {
                stmt: StmtRef::Owned(Some(stmt)),
                bindings: self.bindings,
            })
        }
    }
}

impl<'a, B: Connector> IntoQuery<'a, B> for &'a str
where
    B::Statement: 'static,
    B::Error: core::error::Error,
{
    fn into_query<E>(
        self,
        executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move {
            let stmt = executor.prepare(self).await.map_err(Error::connector)?;
            Ok(Query {
                stmt: StmtRef::Owned(Some(stmt)),
                bindings: Default::default(),
            })
        }
    }
}

impl<'a, B: Connector> IntoQuery<'a, B> for &'a mut Stmt<B>
where
    B::Statement: 'static,
    B::Error: core::error::Error,
{
    fn into_query<E>(
        self,
        _executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move {
            Ok(Query {
                stmt: StmtRef::Borrow(&mut self.0),
                bindings: Default::default(),
            })
        }
    }
}

impl<'a, B: Connector> IntoQuery<'a, B> for Stmt<B>
where
    B::Statement: 'static,
    B::Error: core::error::Error,
{
    fn into_query<E>(
        self,
        _executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move {
            Ok(Query {
                stmt: StmtRef::Owned(Some(self.0)),
                bindings: Default::default(),
            })
        }
    }
}

impl<'a, B: Connector> IntoQuery<'a, B> for CreateTable<'a>
where
    B::Statement: 'static,
    B::Error: core::error::Error,
{
    fn into_query<E>(
        self,
        executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move {
            let stmt = self
                .to_sql(executor.db_info().variant())
                .map_err(Error::query)?;

            stmt.into_query(executor).await
        }
    }
}

impl<'a, B: Connector> IntoQuery<'a, B> for Insert<'a>
where
    B::Statement: 'static,
    B::Error: core::error::Error,
{
    fn into_query<E>(
        self,
        executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move {
            let stmt = self
                .to_sql(executor.db_info().variant())
                .map_err(Error::query)?;

            stmt.into_query(executor).await
        }
    }
}

impl<'a, B: Connector, S> IntoQuery<'a, B> for InsertReturning<'a, S>
where
    B::Statement: 'static,
    B::Error: core::error::Error,
    S: Selection<'a> + Send,
{
    fn into_query<E>(
        self,
        executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move {
            let stmt = self
                .to_sql(executor.db_info().variant())
                .map_err(Error::query)?;

            stmt.into_query(executor).await
        }
    }
}

impl<'a, B: Connector, T> IntoQuery<'a, B> for QueryStmt<T>
where
    T: usql_builder::select::Query<'a> + Send,
    B::Statement: 'static,
    B::Error: core::error::Error,
{
    fn into_query<E>(
        self,
        executor: &E,
    ) -> impl Future<Output = Result<Query<'a, B>, Error<B>>> + Send
    where
        E: Executor<Connector = B> + Send + Sync,
    {
        async move {
            let stmt = self
                .to_sql(executor.db_info().variant())
                .map_err(Error::query)?;

            stmt.into_query(executor).await
        }
    }
}
