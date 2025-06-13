use usql_core::{Connector, Statement};

pub struct Stmt<B: Connector>(B::Statement);

impl<B: Connector> Stmt<B> {
    pub fn finalize(self) {
        self.0.finalize()
    }
}
