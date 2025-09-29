use std::{
    pin::Pin,
    task::{Poll, ready},
};

use futures::Stream;
use usql_core::{Connector, Row as _};
use usql_value::Value;

use crate::{
    Output, RowWriter, Unpack, UnpackError, error::Error, project::Project, result::IntoResult,
    row::Row,
};

impl Project {
    // pub async fn next_row_async<'b, T>(
    //     &'b self,
    //     iter: &mut futures::stream::Peekable<T>,
    // ) -> Option<Result<Row<'b, 'a, <T::Item as IntoResult>::Ok>, Error<>>
    // where
    //     T: Stream + Unpin,
    //     T::Item: IntoResult,
    //     <T::Item as IntoResult>::Ok: usql_core::Row,
    //     <T::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
    //     <<<T::Item as IntoResult>::Ok as usql_core::Row>::Connector as Connector>::Error:
    //         core::error::Error + Send + Sync + 'static,
    // {
    //     let mut iter = Pin::new(iter);

    //     let next = match iter.next().await?.into_result() {
    //         Ok(ret) => ret,
    //         Err(err) => return Some(Err(Error::conn(err))),
    //     };

    //     let pk = match next.get(self.pk) {
    //         Ok(ret) => ret,
    //         Err(err) => return Some(Err(Error::conn(err))),
    //     };

    //     let pk = pk.to_owned();

    //     let mut result = vec![next];

    //     loop {
    //         if let Some(peek) = iter.as_mut().peek().await {
    //             let Ok(ret) = peek.as_result() else {
    //                 break;
    //             };

    //             let next_pk = match ret.get(self.pk) {
    //                 Ok(ret) => ret,
    //                 Err(_) => break,
    //             };

    //             if next_pk.as_ref() != pk.as_ref() {
    //                 break;
    //             }
    //         }

    //         let Some(next) = iter.next().await else {
    //             break;
    //         };

    //         let next = match next.into_result() {
    //             Ok(ret) => ret,
    //             Err(err) => return Some(Err(Error::conn(err))),
    //         };

    //         result.push(next);
    //     }

    //     Some(Ok(Row {
    //         rows: result,
    //         project: self,
    //     }))
    // }

    pub fn wrap_stream<'b, S>(&'b self, stream: S) -> ProjectionStream<'b, S>
    where
        S: Stream + Unpin,
        S::Item: IntoResult,
        <S::Item as IntoResult>::Ok: usql_core::Row,
        <S::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
        <<<S::Item as IntoResult>::Ok as usql_core::Row>::Connector as Connector>::Error:
            core::error::Error + Send + Sync + 'static,
    {
        ProjectionStream {
            stream,
            project: self,
            cache: Default::default(),
            pk: Value::Null,
        }
    }
}

pub struct ProjectionStream<'a, S>
where
    S: Stream + Unpin,
    S::Item: IntoResult,
{
    stream: S,
    project: &'a Project,
    cache: Vec<<S::Item as IntoResult>::Ok>,
    pk: Value,
}

impl<'a, S> ProjectionStream<'a, S>
where
    S: Stream + Unpin,
    S::Item: IntoResult,
    <S::Item as IntoResult>::Ok: usql_core::Row,
    <S::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
    <<<S::Item as IntoResult>::Ok as usql_core::Row>::Connector as Connector>::Error:
        core::error::Error + Send + Sync + 'static,
{
    pub fn unpack<O: Output>(self, output: O) -> WriteTo<'a, S, O> {
        WriteTo {
            stream: self,
            output,
        }
    }
}

impl<'a, S> Stream for ProjectionStream<'a, S>
where
    S: Stream + Unpin,
    S::Item: IntoResult,
    <S::Item as IntoResult>::Ok: usql_core::Row,
    <S::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
    <<<S::Item as IntoResult>::Ok as usql_core::Row>::Connector as Connector>::Error:
        core::error::Error + Send + Sync + 'static,
{
    type Item = Result<
        Row<<S::Item as IntoResult>::Ok>,
        Error<<<S::Item as IntoResult>::Ok as usql_core::Row>::Connector>,
    >;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        loop {
            let this = unsafe { Pin::get_unchecked_mut(self.as_mut()) };

            let ret = match ready!(unsafe { Pin::new_unchecked(&mut this.stream) }.poll_next(cx)) {
                Some(ret) => ret,
                None => {
                    if this.cache.is_empty() {
                        return Poll::Ready(None);
                    }

                    let rows = core::mem::take(&mut this.cache);

                    return Poll::Ready(Some(Ok(Row {
                        rows,
                        project: this.project.clone(),
                    })));
                }
            };

            let ret = match ret.into_result() {
                Ok(ret) => ret,
                Err(err) => return Poll::Ready(Some(Err(UnpackError::new(err).into()))),
            };

            let pk = match ret.get((&this.project.inner().pk).into()) {
                Ok(ret) => ret.to_owned(),
                Err(err) => return Poll::Ready(Some(Err(Error::Connector(err)))),
            };

            if this.pk.is_null() {
                this.pk = pk;
            } else if pk.is_null() || pk != this.pk {
                this.pk = pk;
                let rows = core::mem::replace(&mut this.cache, vec![ret]);
                return Poll::Ready(Some(Ok(Row {
                    rows,
                    project: this.project.clone(),
                })));
            }

            this.cache.push(ret);
        }
    }
}

#[pin_project::pin_project]
pub struct WriteTo<'a, S, O>
where
    S: Stream + Unpin,
    S::Item: IntoResult,
{
    #[pin]
    stream: ProjectionStream<'a, S>,
    output: O,
}

impl<'a, S, O> Stream for WriteTo<'a, S, O>
where
    S: Stream + Unpin,
    S::Item: IntoResult,
    <S::Item as IntoResult>::Ok: usql_core::Row,
    <S::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
    <<<S::Item as IntoResult>::Ok as usql_core::Row>::Connector as Connector>::Error:
        core::error::Error + Send + Sync + 'static,
    O: Output,
{
    type Item = Result<
        <O::Writer as RowWriter>::Output,
        Error<<<S::Item as IntoResult>::Ok as usql_core::Row>::Connector>,
    >;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match ready!(this.stream.poll_next(cx)) {
            Some(Ok(ret)) => {
                let ret = ret.unpack(this.output.create()).map_err(Into::into);
                Poll::Ready(Some(ret))
            }
            Some(Err(err)) => Poll::Ready(Some(Err(err))),
            None => Poll::Ready(None),
        }
    }
}
