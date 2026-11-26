use usql_core::{Connector, Executor};
use usql_value::ValueCow;

use crate::{
    conn::Conn,
    connector::{Info, Mysql},
};

/// Rolled back when dropped without a commit.
///
/// The rollback is sent before the next query on the connection, or by the pool's
/// connection reset when the connection is returned.
pub struct Transaction<'conn> {
    conn: &'conn Conn,
    done: bool,
}

impl<'conn> Transaction<'conn> {
    pub(crate) fn new(conn: &'conn Conn) -> Transaction<'conn> {
        Transaction { conn, done: false }
    }

    async fn finish(mut self, stmt: &'static str) -> Result<(), crate::error::Error> {
        self.conn.exec_batch(stmt).await?;
        self.done = true;
        Ok(())
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.done {
            self.conn.abandon_transaction();
        }
    }
}

impl<'conn> usql_core::Transaction<'conn> for Transaction<'conn> {
    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        self.finish("COMMIT")
    }

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send {
        self.finish("ROLLBACK")
    }
}

impl Executor for Transaction<'_> {
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
        self.conn.prepare(query)
    }

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> usql_core::QueryStream<'a, Self::Connector> {
        self.conn.query(stmt, params)
    }

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        self.conn.exec(stmt, params)
    }

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a {
        self.conn.exec_batch(stmt)
    }
}
