use usql_core::{Connector, DatabaseInfo, System};

use crate::{row::Row, stmt::Stmt};

use super::pool::{Conn, Manager, ManagerOptions, Pool};

pub struct LibSql;

impl Connector for LibSql {
    type Connection = Conn;

    type Statement = Stmt;

    type Row = Row;

    type Info = LibSqlInfo;

    type Pool = Pool;

    type Error = super::error::Error;

    type Options = ManagerOptions;

    fn create_pool(
        options: Self::Options,
    ) -> impl Future<Output = Result<Self::Pool, Self::Error>> + Send {
        async move {
            let manager = Manager::new(options);
            Ok(Pool::new(manager))
        }
    }
}

pub struct LibSqlInfo;

impl DatabaseInfo for LibSqlInfo {
    fn variant(&self) -> usql_core::System {
        System::LibSql
    }
}
