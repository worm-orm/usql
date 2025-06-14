use crate::{IntoQuery, QueryStream, conn::Conn, error::Error};
use alloc::boxed::Box;
use usql_core::{Connector, Pool as _};

pub struct Pool<B: Connector> {
    pool: B::Pool,
}

impl<B: Connector> Pool<B> {
    pub fn new(pool: B::Pool) -> Pool<B> {
        Pool { pool }
    }
}

impl<B: Connector> Pool<B> {
    pub async fn conn(&self) -> Result<Conn<B>, Error<B>> {
        self.pool
            .get()
            .await
            .map(Conn::new)
            .map_err(Error::connector)
    }
}
