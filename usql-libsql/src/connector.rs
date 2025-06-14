use usql_core::{Connector, DatabaseInfo, Statement, System};

use super::pool::{LibSqlPool, Manager, ManagerOptions, PooledConn};

pub struct LibSql;

impl Connector for LibSql {
    type Connection = PooledConn;

    type Statement = libsql::Statement;

    type Row = libsql::Row;

    type Info = LibSqlInfo;

    type Pool = LibSqlPool;

    type Error = super::error::Error;

    type Options = ManagerOptions;

    fn create_pool(
        options: Self::Options,
    ) -> impl Future<Output = Result<Self::Pool, Self::Error>> + Send {
        async move {
            let manager = Manager::new(options);
            Ok(LibSqlPool::new(manager))
        }
    }
}

pub struct LibSqlInfo;

impl DatabaseInfo for LibSqlInfo {
    fn variant(&self) -> crate::system::System {
        System::LibSql
    }
}

impl Statement for libsql::Statement {
    type Connector = LibSql;
}
