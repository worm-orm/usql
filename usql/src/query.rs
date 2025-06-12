use usql_builder::SqlStmt;
use usql_core::{Connector, Executor, ValueCow};

use crate::error::Error;

pub(crate) enum StmtRef<'a, B: Connector> {
    Borrow(&'a mut B::Statement),
    Owned(B::Statement),
}

impl<'a, B: Connector> StmtRef<'a, B> {
    pub fn as_mut(&mut self) -> &mut B::Statement {
        match self {
            StmtRef::Borrow(stmt) => stmt,
            StmtRef::Owned(stmt) => stmt,
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
                stmt: StmtRef::Owned(stmt),
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
                stmt: StmtRef::Owned(stmt),
                bindings: Default::default(),
            })
        }
    }
}
