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

impl<B: Connector> AsRef<B::Statement> for Stmt<B> {
    fn as_ref(&self) -> &B::Statement {
        &self.0
    }
}

impl<B: Connector> AsMut<B::Statement> for Stmt<B> {
    fn as_mut(&mut self) -> &mut B::Statement {
        &mut self.0
    }
}
