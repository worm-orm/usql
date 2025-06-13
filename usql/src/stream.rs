use futures_core::{Stream, stream::BoxStream};
use pin_project_lite::pin_project;
use usql_core::Connector;

use crate::{error::Error, row::Row};

pin_project! {
  pub struct QueryStream<'a, B: Connector> {
    #[pin]
    pub(crate)stream: BoxStream<'a, Result<Row<B>, Error<B>>>,
  }
}

impl<'a, B> Stream for QueryStream<'a, B>
where
    B: Connector,
{
    type Item = Result<Row<B>, Error<B>>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();
        this.stream.poll_next(cx)
    }
}
