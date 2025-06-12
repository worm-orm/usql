use futures_core::future::BoxFuture;
use usql::{Connection, Connector, Executor};

use crate::{error::Error, exec::Exec, loader::MigrationLoader};

pub struct Migration<T> {
    pub name: String,
    pub runner: T,
}

pub trait Runner<B: Connector> {
    type Error;

    fn up<'a>(
        &'a self,
        executor: &'a Exec<'_, B>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'a;

    fn down<'a>(
        &'a self,
        executor: &'a Exec<'_, B>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'a;
}

pub trait DynamicRunner<B: Connector>: Send + Sync {
    fn up<'a>(&'a self, conn: &'a Exec<'_, B>) -> BoxFuture<'a, Result<(), Error>>;

    fn down<'a>(&'a self, conn: &'a Exec<'_, B>) -> BoxFuture<'a, Result<(), Error>>;
}

impl<B> Runner<B> for Box<dyn DynamicRunner<B>>
where
    B: Connector,
    for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
{
    type Error = Error;

    fn up<'a>(
        &'a self,
        executor: &'a Exec<'_, B>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'a {
        async move { (**self).up(executor).await }
    }

    fn down<'a>(
        &'a self,
        executor: &'a Exec<'_, B>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'a {
        async move { (**self).down(executor).await }
    }
}

pub fn runner_box<M, B>(migration: M) -> Box<dyn DynamicRunner<B>>
where
    B: Connector,
    for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
    M: Runner<B> + Send + Sync + 'static,
    M::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    Box::new(MigrationBox(migration))
}

struct MigrationBox<T>(T);

impl<T, B> DynamicRunner<B> for MigrationBox<T>
where
    B: Connector,
    for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
    T: Runner<B> + Send + Sync,
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    fn up<'a>(&'a self, conn: &'a Exec<'_, B>) -> BoxFuture<'a, Result<(), Error>> {
        Box::pin(async move { self.0.up(conn).await.map_err(Error::new) })
    }

    fn down<'a>(&'a self, conn: &'a Exec<'_, B>) -> BoxFuture<'a, Result<(), Error>> {
        Box::pin(async move { self.0.down(conn).await.map_err(Error::new) })
    }
}
