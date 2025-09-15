use futures::TryStreamExt;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

// use usql::{
//     Conn, Error,
//     core::{Connection, Connector, Executor},
//     value::chrono::Utc,
// };

use chrono::Utc;
use usql_core::{Connection, Connector, Executor, Pool, Transaction};

use crate::{
    data::{Entry, ensure_table, get_entry, insert_migration, list_entries},
    error::Error,
    exec::Exec,
    loader::MigrationLoader,
    migration::{Migration, MigrationInfo, Runner},
};

pub struct Migrator<B, T>
where
    B: Connector,
{
    pool: B::Pool,
    loader: T,
    path: PathBuf,
    table_name: String,
}

impl<B, T> Migrator<B, T>
where
    B: Connector + 'static,
    B::Error: Into<Box<dyn core::error::Error + Send + Sync>>
        + core::error::Error
        + Send
        + Sync
        + 'static,
    B::Connection: Send,
    for<'b> <B::Connection as Connection>::Transaction<'b>: Send + Sync,
    T: MigrationLoader<B>,
    <T::Migration as Runner<B>>::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    pub fn new(pool: B::Pool, loader: T, path: PathBuf, table_name: String) -> Migrator<B, T> {
        Migrator {
            pool,
            loader,
            path,
            table_name,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn has_migrations(&self) -> Result<bool, Error<B>> {
        let migrations = self.load_migrations().await?;
        let conn = self.pool.get().await.map_err(Error::Connector)?;
        let entries = self.load_entries(&conn).await?;
        let ret = if entries.len() > migrations.len() {
            false
        } else if entries.len() == migrations.len() {
            false
        } else {
            true
        };

        Ok(ret)
    }

    pub async fn list_migrations(&self) -> Result<Vec<Migration<T::Migration>>, Error<B>> {
        let conn = self.pool.get().await.map_err(Error::Connector)?;

        ensure_table(&conn, &self.table_name).await?;

        let info = self.load_migrations().await?;

        let mut output = Vec::with_capacity(info.len());
        for info in info {
            let entry = get_entry(&conn, &self.table_name, &info.name).await?;
            output.push(Migration {
                name: info.name,
                runner: info.runner,
                applied: entry.map(|m| m.date),
            });
        }

        Ok(output)
    }

    pub async fn migrate(&self) -> Result<bool, Error<B>> {
        let migrations = self.load_migrations().await?;
        let mut conn = self.pool.get().await.map_err(Error::Connector)?;
        let ret = self.migration_one(&mut conn, &migrations).await?;
        Ok(ret)
    }

    pub async fn migrate_all(&self) -> Result<bool, Error<B>> {
        let migrations = self.load_migrations().await?;
        let mut conn = self.pool.get().await.map_err(Error::Connector)?;
        let mut ret = false;
        loop {
            if !self.migration_one(&mut conn, &migrations).await? {
                break;
            }
            ret = true;
        }

        Ok(ret)
    }
}

impl<B, T> Migrator<B, T>
where
    B: Connector + 'static,
    B::Statement: 'static,
    T: MigrationLoader<B>,
    <T::Migration as Runner<B>>::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    B::Connection: Send,
    B::Error: Into<Box<dyn core::error::Error + Send + Sync>>
        + core::error::Error
        + Send
        + Sync
        + 'static,
    for<'b> <B::Connection as Connection>::Transaction<'b>: Send + Sync,
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    async fn migration_one(
        &self,
        conn: &mut B::Connection,
        migrations: &Vec<MigrationInfo<T::Migration>>,
    ) -> Result<bool, Error<B>> {
        let trans = conn.begin().await.map_err(Error::Connector)?;

        let entries = self.load_entries(&trans).await?;

        if entries.len() > migrations.len() {
            panic!("Invalid state")
        } else if entries.len() == migrations.len() {
            return Ok(false);
        }

        for (entry, migration) in entries.iter().zip(migrations.iter().take(entries.len())) {
            if entry.name != migration.name {
                panic!("invalid state")
            }
        }

        let migration = &migrations[entries.len()];

        let exec = Exec::new(trans);

        migration.runner.up(&exec).await.map_err(Error::load)?;

        insert_migration(
            &exec,
            &self.table_name,
            &migration.name,
            Utc::now().naive_utc(),
        )
        .await?;

        exec.conn.commit().await.map_err(Error::Connector)?;

        Ok(true)
    }

    async fn load_migrations(&self) -> Result<Vec<MigrationInfo<T::Migration>>, Error<B>> {
        let readdir = tokio::fs::read_dir(&self.path).await.map_err(Error::load)?;

        let mut readdir = tokio_stream::wrappers::ReadDirStream::new(readdir)
            .map_ok(|m| m.path())
            .try_collect::<Vec<_>>()
            .await
            .map_err(Error::load)?;

        readdir.sort();

        let mut migrations = Vec::with_capacity(readdir.len());

        let mut seen = HashSet::new();

        for path in readdir {
            if !self.loader.can_load(&path).await {
                continue;
            }

            let name = path.file_stem().unwrap().to_string_lossy().to_string();

            if seen.contains(&name) {
                return Err(Error::load(format!("Migration '{}' already found", name)));
            }

            let runner = self.loader.load(&path).await.map_err(Error::load)?;

            seen.insert(name.clone());

            migrations.push(MigrationInfo { name, runner });
        }

        Ok(migrations)
    }

    async fn load_entries<E>(&self, conn: &E) -> Result<Vec<Entry>, Error<B>>
    where
        E: Executor<Connector = B>,
        <E::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
    {
        // TODO: Fix this
        ensure_table(conn, &self.table_name).await?;
        list_entries(conn, &self.table_name).await
    }
}
