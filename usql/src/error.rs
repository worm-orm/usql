use usql_core::Connector;

#[derive(Debug)]
pub enum Error<B: Connector> {
    Connector(B::Error),
    NotFound,
}

impl<B: Connector> Error<B> {
    pub fn connector(error: B::Error) -> Error<B> {
        Error::Connector(error)
    }

    pub fn query<E>(error: E) -> Error<B> {
        todo!()
    }
}
