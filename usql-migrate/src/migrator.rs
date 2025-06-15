use std::{collections::HashSet, path::PathBuf};

use usql_core::{Connection, Connector, Executor, Pool, Transaction};
use usql_value::chrono::Utc;

use crate::{
    data::{Entry, ensure_table, insert_migration, list_entries},
    error::Error,
    exec::Exec,
    loader::MigrationLoader,
    migration::{Migration, Runner},
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
    B: Connector,
    B::Error: Into<Box<dyn core::error::Error + Send + Sync>>
        + core::error::Error
        + Send
        + Sync
        + 'static,
    B::Connection: Send,
    for<'b> <B::Connection as Connection>::Transaction<'b>: Send + Sync,
    T: MigrationLoader<B>,
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    pub fn new(pool: B::Pool, loader: T, path: PathBuf) -> Migrator<B, T> {
        Migrator {
            pool,
            loader,
            path,
            table_name: "migrations".to_string(),
        }
    }

    pub async fn has_migrations(&self) -> Result<bool, Error> {
        let migrations = self.load_migrations().await?;
        let conn = self.pool.get().await.map_err(Error::new)?;
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

    pub async fn migrate(&self) -> Result<(), Error> {
        let migrations = self.load_migrations().await?;
        let mut conn = self.pool.get().await.map_err(Error::new)?;
        self.migration_one(&mut conn, &migrations).await?;
        Ok(())
    }

    pub async fn migrate_all(&self) -> Result<(), Error> {
        let migrations = self.load_migrations().await?;
        let mut conn = self.pool.get().await.map_err(Error::new)?;
        loop {
            if !self.migration_one(&mut conn, &migrations).await? {
                break;
            }
        }

        Ok(())
    }
}

impl<B, T> Migrator<B, T>
where
    B: Connector,
    T: MigrationLoader<B>,
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
        migrations: &Vec<Migration<T::Migration>>,
    ) -> Result<bool, Error> {
        let trans = conn.begin().await.map_err(Error::new)?;

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

        migration.runner.up(&exec).await.map_err(Error::new)?;

        insert_migration(
            &exec,
            &self.table_name,
            &migration.name,
            Utc::now().naive_utc(),
        )
        .await?;

        exec.conn.commit().await.map_err(Error::new)?;

        Ok(true)
    }

    async fn load_migrations(&self) -> Result<Vec<Migration<T::Migration>>, Error> {
        let mut readdir = std::fs::read_dir(&self.path)
            .unwrap()
            .into_iter()
            .map(|m| m.map(|m| m.path()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        readdir.sort();

        let mut migrations = Vec::with_capacity(readdir.len());

        let mut seen = HashSet::new();

        for path in readdir {
            if !self.loader.can_load(&path).await {
                continue;
            }

            let name = path.file_stem().unwrap().to_string_lossy().to_string();

            if seen.contains(&name) {
                panic!("Migration is already defined")
            }

            let runner = self.loader.load(&path).await.map_err(Error::new)?;

            seen.insert(name.clone());

            migrations.push(Migration { name, runner });
        }

        Ok(migrations)
    }

    async fn load_entries<E>(&self, conn: &E) -> Result<Vec<Entry>, Error>
    where
        E: Executor<Connector = B>,
        <E::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
    {
        // TODO: Fix this
        ensure_table(conn, &self.table_name).await?;
        list_entries(conn, &self.table_name).await
    }
}
