use crate::{Connector, DatabaseInfo, Statement, system::System};

pub struct Postgres;

impl Connector for Postgres {
    type Connection = deadpool_postgres::Object;

    type Statement = tokio_postgres::Statement;

    type Row = tokio_postgres::Row;

    type Info = PostgresInfo;

    type Pool = deadpool_postgres::Pool;

    type Error = tokio_postgres::Error;

    type Options = deadpool_postgres::Config;

    fn create_pool(
        options: Self::Options,
    ) -> impl Future<Output = Result<Self::Pool, Self::Error>> + Send {
        async move { todo!() }
    }
}

pub struct PostgresInfo;

impl DatabaseInfo for PostgresInfo {
    fn variant(&self) -> crate::system::System {
        System::Postgres
    }
}

impl Statement for tokio_postgres::Statement {
    type Connector = Postgres;
}
