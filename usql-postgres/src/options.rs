use deadpool_postgres::{Config, CreatePoolError};
use tokio_postgres::NoTls;

use crate::pool::Pool;

pub enum Tls {
    NoTls,
}

pub struct Options {
    tls: Tls,
    config: Config,
}

impl Options {
    pub(crate) fn create_pool(self) -> Result<Pool, CreatePoolError> {
        match self.tls {
            Tls::NoTls => self.config.create_pool(None, NoTls).map(Pool),
        }
    }
}
