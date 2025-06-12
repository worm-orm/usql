use futures_core::future::BoxFuture;
use usql::{Connection, Connector, Executor};

use crate::{error::Error, loader::MigrationLoader};

pub struct Migration<B> {
    name: String,
    runner: Box<dyn DynamicRunner<B>>,
}

pub trait Runner<B> {
    type Loader: MigrationLoader<B>;

    fn up<'a, E>(
        &'a self,
        executor: &'a mut E,
    ) -> impl Future<Output = Result<(), <Self::Loader as MigrationLoader<B>>::Error>> + Send + 'a
    where
        E: Executor<Connector = B>;

    fn down<'a, E>(
        &'a self,
        executor: &'a mut E,
    ) -> impl Future<Output = Result<(), <Self::Loader as MigrationLoader<B>>::Error>> + Send + 'a
    where
        E: Executor<Connector = B>;
}

pub trait DynamicRunner<B: Connector> {
    fn up<'a>(
        &'a self,
        conn: &mut <B::Connection as Connection>::Transaction<'_>,
    ) -> BoxFuture<'a, Result<(), Error>>;

    fn down<'a>(
        &'a self,
        conn: &mut <B::Connection as Connection>::Transaction<'_>,
    ) -> BoxFuture<'a, Result<(), Error>>;
}
