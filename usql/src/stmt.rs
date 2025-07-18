use usql_core::{Connector, Statement};

use crate::Error;

pub struct Stmt<B: Connector>(pub(crate) B::Statement);

impl<B: Connector> Stmt<B> {
    pub fn new(stmt: B::Statement) -> Stmt<B> {
        Self(stmt)
    }
}

impl<B: Connector> Stmt<B> {
    pub fn finalize(self) -> Result<(), Error<B>> {
        self.0.finalize().map_err(Error::connector)
    }
}
