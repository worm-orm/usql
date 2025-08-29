use usql_core::{Connector, DatabaseInfo, System};

use crate::{conn::Conn, error::Error, options::Options, pool::Pool, row::Row, stmt::Statement};

pub struct Postgres;

impl Connector for Postgres {
    type Connection = Conn;

    type Statement = Statement;

    type Row = Row;

    type Info = Info;

    type Pool = Pool;

    type Error = Error;

    type Options = Options;

    fn create_pool(
        options: Self::Options,
    ) -> impl Future<Output = Result<Self::Pool, Self::Error>> + Send {
        async move {
            let pool = options.create_pool()?;
            Ok(pool)
        }
    }
}

pub struct Info;

impl DatabaseInfo for Info {
    fn variant(&self) -> usql_core::System {
        System::Postgres
    }
}
