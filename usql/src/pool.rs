use crate::{conn::Conn, error::Error};
use usql_core::{Connector, Pool as _};

pub struct Pool<B: Connector> {
    pool: B::Pool,
}

impl<B: Connector> Clone for Pool<B>
where
    B::Pool: Clone,
{
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
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
