use std::{
    any::Any,
    boxed::Box,
    collections::HashMap,
    path::PathBuf,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use futures_channel::oneshot;
use rusqlite::types::Value;

use super::{error::Error, query_result::QueryResult, row::Row};

pub enum Request {
    Exec {
        stmt: String,
        values: Vec<rusqlite::types::Value>,
        returns: oneshot::Sender<Result<QueryResult, rusqlite::Error>>,
    },
    ExecBatch {
        stmt: String,
        returns: oneshot::Sender<Result<(), rusqlite::Error>>,
    },
    Fetch {
        stmt: String,
        values: Vec<rusqlite::types::Value>,
        returns: flume::Sender<Result<Row, rusqlite::Error>>,
    },
    Begin {
        channel: flume::Receiver<TransRequest>,
        ready: oneshot::Sender<Result<(), rusqlite::Error>>,
    },
    With {
        #[allow(clippy::complexity)]
        func: Box<
            dyn FnOnce(&rusqlite::Connection) -> Result<Box<dyn Any + Send>, rusqlite::Error>
                + Send,
        >,
        returns: oneshot::Sender<Result<Box<dyn Any + Send>, rusqlite::Error>>,
    },
}

pub enum TransRequest {
    Exec {
        stmt: String,
        values: Vec<rusqlite::types::Value>,
        returns: oneshot::Sender<Result<QueryResult, rusqlite::Error>>,
    },
    ExecBatch {
        stmt: String,
        returns: oneshot::Sender<Result<(), rusqlite::Error>>,
    },
    Fetch {
        stmt: String,
        values: Vec<rusqlite::types::Value>,
        returns: flume::Sender<Result<Row, rusqlite::Error>>,
    },
    Commit {
        returns: oneshot::Sender<Result<(), rusqlite::Error>>,
    },
    Rollup {
        returns: oneshot::Sender<Result<(), rusqlite::Error>>,
    },
}

pub async fn open_worker(
    flags: rusqlite::OpenFlags,
    path: Option<PathBuf>,
) -> Result<flume::Sender<Request>, Error> {
    let (sx, rx) = flume::bounded(1);
    let (on_open, wait) = oneshot::channel();

    std::thread::spawn(move || worker(rx, on_open, flags, path));

    wait.await.map_err(|_| Error::Channel)??;

    Ok(sx)
}

pub fn worker(
    rx: flume::Receiver<Request>,
    on_open: oneshot::Sender<Result<(), rusqlite::Error>>,
    flags: rusqlite::OpenFlags,
    path: Option<PathBuf>,
) {
    let conn = if let Some(path) = path {
        rusqlite::Connection::open_with_flags(path, flags)
    } else {
        rusqlite::Connection::open_in_memory_with_flags(flags)
    };

    let mut client = match conn {
        Ok(client) => {
            on_open.send(Ok(())).ok();
            client
        }
        Err(err) => {
            on_open.send(Err(err)).ok();
            return;
        }
    };

    while let Ok(next) = rx.recv() {
        match next {
            Request::Exec {
                stmt,
                values,
                returns,
            } => {
                returns.send(execute(&client, stmt, values)).ok();
            }
            Request::ExecBatch { stmt, returns } => {
                returns.send(execute_batch(&client, stmt)).ok();
            }
            Request::Fetch {
                stmt,
                values,
                returns,
            } => {
                fetch(&client, stmt, values, returns);
            }
            Request::Begin { channel, ready } => {
                let trans = match client.transaction() {
                    Ok(ret) => {
                        ready.send(Ok(())).ok();
                        ret
                    }
                    Err(err) => {
                        ready.send(Err(err)).ok();
                        continue;
                    }
                };

                transaction(trans, channel);
            }
            Request::With { func, returns } => {
                returns.send(func(&client)).ok();
            }
        };
    }
}

fn execute<C: SqliteConn>(
    conn: &C,
    stmt: String,
    params: Vec<Value>,
) -> Result<QueryResult, rusqlite::Error> {
    let stmt = conn.execute(&stmt, rusqlite::params_from_iter(params))?;

    let last_insert_id = conn.last_insert_rowid();

    Ok(QueryResult {
        last_insert_id,
        affected_rows: stmt,
    })
}

fn execute_batch<C: SqliteConn>(conn: &C, stmt: String) -> Result<(), rusqlite::Error> {
    conn.execute_batch(&stmt)
}

fn fetch<C: SqliteConn>(
    conn: &C,
    stmt: String,
    params: Vec<Value>,
    returns: flume::Sender<Result<Row, rusqlite::Error>>,
) {
    let params = rusqlite::params_from_iter(params);

    let mut stmt = match conn.prepare(&stmt) {
        Ok(ret) => ret,
        Err(err) => {
            returns.send(Err(err)).ok();
            return;
        }
    };

    let mut rows = match stmt.query(params) {
        Ok(ret) => ret,
        Err(err) => {
            returns.send(Err(err)).ok();
            return;
        }
    };

    let column_count = rows.as_ref().unwrap().column_count();

    let columns = (0..column_count)
        .map(|m| {
            (
                rows.as_ref()
                    .unwrap()
                    .column_name(m)
                    .expect("col")
                    .to_string(),
                m,
            )
        })
        .collect::<HashMap<_, _>>();

    let columns = Arc::new(columns);

    loop {
        let next = match rows.next() {
            Ok(Some(next)) => next,
            Ok(None) => return,
            Err(err) => {
                returns.send(Err(err)).ok();
                return;
            }
        };

        let values = (0..column_count)
            .map(|m| {
                let val: rusqlite::types::Value = next.get_ref(m).expect("msg").into();
                val
            })
            .collect();

        if returns
            .send(Ok(Row {
                values,
                columns: columns.clone(),
            }))
            .is_err()
        {
            break;
        }
    }
}

fn transaction(conn: rusqlite::Transaction<'_>, channel: flume::Receiver<TransRequest>) {
    while let Ok(next) = channel.recv() {
        match next {
            TransRequest::Exec {
                stmt,
                values,
                returns,
            } => {
                returns.send(execute(&conn, stmt, values)).ok();
            }
            TransRequest::ExecBatch { stmt, returns } => {
                returns.send(execute_batch(&conn, stmt)).ok();
            }
            TransRequest::Fetch {
                stmt,
                values,
                returns,
            } => {
                fetch(&conn, stmt, values, returns);
            }
            TransRequest::Commit { returns } => {
                returns.send(conn.commit()).ok();
                return;
            }
            TransRequest::Rollup { returns } => {
                returns.send(conn.rollback()).ok();
                return;
            }
        }
    }

    conn.rollback().ok();
}

trait SqliteConn {
    fn execute_batch(&self, sql: &str) -> Result<(), rusqlite::Error>;
    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<usize, rusqlite::Error>;
    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, rusqlite::Error>;
    fn last_insert_rowid(&self) -> i64;
}

impl SqliteConn for rusqlite::Connection {
    fn execute_batch(&self, sql: &str) -> Result<(), rusqlite::Error> {
        self.execute_batch(sql)
    }

    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<usize, rusqlite::Error> {
        self.execute(sql, params)
    }

    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, rusqlite::Error> {
        self.prepare(sql)
    }

    fn last_insert_rowid(&self) -> i64 {
        self.last_insert_rowid()
    }
}

impl SqliteConn for rusqlite::Transaction<'_> {
    fn execute_batch(&self, sql: &str) -> Result<(), rusqlite::Error> {
        (**self).execute_batch(sql)
    }
    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<usize, rusqlite::Error> {
        (**self).execute(sql, params)
    }

    fn prepare(&self, sql: &str) -> Result<rusqlite::Statement<'_>, rusqlite::Error> {
        (**self).prepare(sql)
    }

    fn last_insert_rowid(&self) -> i64 {
        (**self).last_insert_rowid()
    }
}
