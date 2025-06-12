use usql::{Connector, Executor};

use crate::{error::Error, loader::MigrationLoader};

pub struct Migrator<B, T>
where
    B: Connector,
{
    pool: B::Pool,
    loader: T,
}

impl<B, T> Migrator<B, T>
where
    B: Connector,
    T: MigrationLoader<B>,
{
    pub const fn new(pool: B::Pool, loader: T) -> Migrator<B, T> {
        Migrator { pool, loader }
    }

    pub async fn migrate(&self) -> Result<bool, Error> {
        todo!()
    }
}
