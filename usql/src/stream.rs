use core::{marker::PhantomData, task::Poll};

use futures_core::{Stream, ready, stream::BoxStream};
use pin_project_lite::pin_project;
use usql_core::Connector;

use crate::{FromRow, error::Error, row::Row};

pin_project! {
  pub struct QueryStream<'a, B: Connector> {
    #[pin]
    pub(crate)stream: BoxStream<'a, Result<Row<B>, Error<B>>>,
  }
}

impl<'a, B: Connector> QueryStream<'a, B> {
    pub fn into<T>(self) -> FromRowStream<'a, B, T>
    where
        T: FromRow,
    {
        FromRowStream {
            stream: self,
            data: PhantomData,
        }
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

pin_project! {
    pub struct FromRowStream<'a, B: Connector, T> {
        #[pin]
        stream: QueryStream<'a, B>,
        data: PhantomData<fn () -> T>
    }
}

impl<'a, B, T> Stream for FromRowStream<'a, B, T>
where
    B: Connector,
    T: FromRow,
{
    type Item = Result<T, Error<B>>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();

        match ready!(this.stream.poll_next(cx)) {
            Some(Ok(row)) => Poll::Ready(Some(T::from_row(row))),
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
    }
}
