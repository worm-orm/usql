use std::task::Poll;

use futures_core::{Stream, ready, stream::BoxStream};
use pin_project_lite::pin_project;
use usql_core::Connector;

use crate::{error::Error, row::Row};

pin_project! {
  pub struct QueryStream<'a, B: Connector> {
    #[pin]
    pub(crate)stream: BoxStream<'a, Result<B::Row, B::Error>>,
  }
}

impl<'a, B> Stream for QueryStream<'a, B>
where
    B: Connector,
{
    type Item = Result<Row<B>, Error<B>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();

        let ret = match ready!(this.stream.poll_next(cx)) {
            Some(Ok(row)) => Some(Ok(Row { row })),
            Some(Err(err)) => Some(Err(Error::Connector(err))),
            None => None,
        };

        Poll::Ready(ret)
    }
}
