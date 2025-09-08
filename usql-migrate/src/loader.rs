use std::path::Path;

use usql_core::{Connection, Connector};

use crate::migration::{DynamicRunner, Runner, runner_box};

pub trait MigrationLoader<B: Connector> {
    type Migration: Runner<B>;
    type Error;

    fn can_load<'a>(&'a self, path: &'a Path) -> impl Future<Output = bool> + Send + 'a;

    fn load<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<Self::Migration, Self::Error>> + Send + 'a;
}

macro_rules! loaders {
    ($only: ident) => {
        impl<B: Connector, $only> MigrationLoader<B> for ($only,)
        where
            for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
            $only: MigrationLoader<B> + Send + Sync,
            $only::Migration: Send + Sync + 'static,
            <$only::Migration as Runner<B>>::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
            $only::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
        {
            type Migration = Box<dyn DynamicRunner<B>>;
            type Error = B::Error;

            fn can_load<'a>(&'a self, path: &'a Path) -> impl Future<Output = bool> + Send + 'a {
                async move { self.0.can_load(path).await }
            }

            fn load<'a>(
                &'a self,
                path: &'a Path,
            ) -> impl Future<Output = Result<Self::Migration, Self::Error>> + Send + 'a {
                async move {
                    let migration = self.0.load(path).await?;
                    Ok(runner_box(migration))
                 }
            }
        }
    };
    ($first: ident, $($rest: ident),+) => {
        loaders!($($rest),+);

        #[allow(non_snake_case)]
        impl<B: Connector, $first, $($rest),+> MigrationLoader<B> for ($first,$($rest),+)
        where
            for<'a> <B::Connection as Connection>::Transaction<'a>: Send + Sync,
            $first: MigrationLoader<B> + Send + Sync,
            $first::Migration: Send + Sync + 'static,
            <$first::Migration as Runner<B>>::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
            $first::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
            $(
                $rest: MigrationLoader<B> + Send + Sync,
                $rest::Migration: Send + Sync + 'static,
                <$rest::Migration as Runner<B>>::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
                $rest::Error: Into<Box<dyn core::error::Error + Send + Sync>>
            ),+
        {
            type Migration = Box<dyn DynamicRunner<B>>;
            type Error = B::Error;

            fn can_load<'a>(&'a self, path: &'a Path) -> impl Future<Output = bool> + Send + 'a {
                async move {
                    let ($first, $($rest),+) = self;
                    $first.can_load(path).await || $($rest.can_load(path).await)||+
                }
            }

            fn load<'a>(
                &'a self,
                path: &'a Path,
            ) -> impl Future<Output = Result<Self::Migration, Self::Error>> + Send + 'a {
                async move {
                    let ($first, $($rest),+) = self;
                    if $first.can_load(path).await {
                        return $first.load(path).await.map_err(Error::unknown).map(runner_box);

                    }
                    $(
                        if $rest.can_load(path).await {
                            return $rest.load(path).await.map_err(Error::unknown).map(runner_box);
                        }
                    )+

                    Err(Error::unknown("Invalid loader"))
                 }
            }
        }
    };
}

loaders!(T1, T2, T3, T4, T5, T6);
