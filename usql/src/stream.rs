use core::{marker::PhantomData, task::Poll};
use futures::TryStreamExt;
use futures_core::{Stream, ready, stream::BoxStream};
use pin_project_lite::pin_project;
use usql_core::Connector;
use usql_util::{Output, Project, Writer};

use crate::{FromRow, error::Error, row::Row};

pin_project! {
  pub struct QueryStream<'a, B: Connector> {
    #[pin]
    pub(crate)stream: BoxStream<'a, Result<Row<B>, Error<B>>>,
  }
}

impl<'a, B: Connector> QueryStream<'a, B> {
    pub fn new(stream: BoxStream<'a, Result<Row<B>, Error<B>>>) -> QueryStream<'a, B> {
        QueryStream { stream }
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

    pub fn project_into<O>(self, project: Project<'a>, output: O) -> ProjectStream<'a, O>
    where
        B: 'static,
        O: Output + Send + Sync + 'a,
        O::Writer: Send,
        <O::Writer as Writer>::Output: Send,
        <O::Writer as Writer>::Error: core::error::Error + Send + Sync + 'static,
        B::Error: core::error::Error + Send + Sync + 'static,
    {
        let stream = project.wrap_stream(output, self.stream.map_ok(|m| m.into_inner()));
        ProjectStream { stream }
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

pin_project! {
    pub struct ProjectStream<'a, O>
where
    O: Output,
{
    #[pin]
    stream: BoxStream<'a, usql_util::anyhow::Result<<O::Writer as Writer>::Output>>,
}
}

impl<'a, O> Stream for ProjectStream<'a, O>
where
    O: Output,
{
    type Item = usql_util::anyhow::Result<<O::Writer as Writer>::Output>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
