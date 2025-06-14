use super::{
    error::Error,
    pool::{Manager, ManagerOptions, Pool, PooledConn},
    row::Row,
    statement::Statement,
};
use usql_core::{Connector, DatabaseInfo, System};
pub struct Sqlite;

impl Connector for Sqlite {
    type Connection = PooledConn;

    type Statement = Statement;

    type Row = Row;

    type Info = SqliteDatabaseInfo;

    type Pool = Pool;

    type Error = Error;

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

pub struct SqliteDatabaseInfo;

impl DatabaseInfo for SqliteDatabaseInfo {
    fn variant(&self) -> usql_core::System {
        System::Sqlite
    }
}
