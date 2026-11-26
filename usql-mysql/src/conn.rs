use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use async_lock::{Mutex, MutexGuard};
use mysql_async::{Params, prelude::Queryable};
use usql_core::{Connection, Connector, Executor};
use usql_value::ValueCow;

use crate::{
    connector::{Info, Mysql},
    convert::to_param,
    error::Error,
    row::Row,
    stmt::Statement,
    transaction::Transaction,
};

/// A pooled connection.
///
/// mysql_async needs exclusive access to run a query, so queries on the same
/// connection wait for each other.
pub struct Conn {
    inner: Mutex<mysql_async::Conn>,
    /// Set when a transaction is dropped without a commit or rollback
    rollback: AtomicBool,
    /// Changed when the default database may have changed, see [Conn::prepare]
    generation: AtomicU64,
}

impl Conn {
    pub(crate) fn new(conn: mysql_async::Conn) -> Conn {
        Conn {
            inner: Mutex::new(conn),
            rollback: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        }
    }

    /// Lock the connection, after rolling back an abandoned transaction
    pub(crate) async fn lock(&self) -> Result<MutexGuard<'_, mysql_async::Conn>, Error> {
        let mut conn = self.inner.lock().await;
        if self.rollback.swap(false, Ordering::AcqRel) {
            if let Err(err) = conn.query_drop("ROLLBACK").await {
                self.rollback.store(true, Ordering::Release);
                return Err(err.into());
            }
        }
        Ok(conn)
    }

    pub(crate) fn abandon_transaction(&self) {
        self.rollback.store(true, Ordering::Release);
    }

    /// A statement uses the default database of when it was prepared, also for `DATABASE()`.
    /// mysql_async caches statements by their text, so the text is changed after a `USE`.
    pub(crate) async fn prepare(&self, query: &str) -> Result<Statement, Error> {
        let mut conn = self.lock().await?;
        let stmt = match self.generation.load(Ordering::Acquire) {
            0 => conn.prep(query).await?,
            generation => {
                conn.prep(format!("/* usql:{generation} */ {query}"))
                    .await?
            }
        };
        Ok(Statement(stmt))
    }

    pub(crate) fn query<'a>(
        &'a self,
        stmt: &'a Statement,
        params: Vec<ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Mysql> {
        let stream = async_stream::try_stream! {
            let params = params_of(stmt, params)?;
            let mut conn = self.lock().await?;
            let mut result = conn.exec_iter(&stmt.0, params).await?;

            while let Some(row) = result.next().await? {
                yield Row(row);
            }

            result.drop_result().await?;
        };

        Box::pin(stream)
    }

    pub(crate) async fn exec(
        &self,
        stmt: &Statement,
        params: Vec<ValueCow<'_>>,
    ) -> Result<(), Error> {
        let params = params_of(stmt, params)?;
        let mut conn = self.lock().await?;
        conn.exec_drop(&stmt.0, params).await?;
        Ok(())
    }

    /// Runs on the text protocol, which allows several statements
    pub(crate) async fn exec_batch(&self, stmt: &str) -> Result<(), Error> {
        let mut conn = self.lock().await?;
        let result = conn.query_drop(stmt).await;
        // Only `USE` changes the default database, and it can't be prepared
        if stmt.to_ascii_lowercase().contains("use") {
            self.generation.fetch_add(1, Ordering::AcqRel);
        }
        Ok(result?)
    }
}

fn params_of(stmt: &Statement, params: Vec<ValueCow<'_>>) -> Result<Params, Error> {
    // mysql_async closes the connection on this error, so check it first
    let expected = stmt.0.num_params().into();
    if params.len() != expected {
        return Err(Error::ParamCount {
            expected,
            found: params.len(),
        });
    }
    if params.is_empty() {
        return Ok(Params::Empty);
    }
    params
        .iter()
        .map(|value| to_param(value.as_ref()))
        .collect::<Result<Vec<_>, _>>()
        .map(Params::Positional)
}

impl Executor for Conn {
    type Connector = Mysql;

    fn db_info(&self) -> <Self::Connector as Connector>::Info {
        Info
    }

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
        Conn::prepare(self, query)
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        Conn::query(self, stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        Conn::exec(self, stmt, params)
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        Conn::exec_batch(self, stmt)
    }
}

impl Connection for Conn {
    type Transaction<'conn>
        = Transaction<'conn>
    where
        Self: 'conn;

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, <Self::Connector as Connector>::Error>> + Send
    {
        async move {
            self.exec_batch("START TRANSACTION").await?;
            Ok(Transaction::new(self))
        }
    }
}
