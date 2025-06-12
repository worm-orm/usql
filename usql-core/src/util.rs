use core::pin::Pin;

use futures_core::Stream;
use pin_project_lite::pin_project;

pub fn next<T>(stream: &mut T) -> Next<'_, T> {
    Next::new(stream)
}

pin_project! {
pub struct Next<'a, T: ?Sized> {
  stream: &'a mut T
}
}

impl<'a, T> Next<'a, T> {
    pub fn new(stream: &'a mut T) -> Next<'a, T> {
        Next { stream }
    }
}

impl<T: ?Sized + Stream + Unpin> Future for Next<'_, T>
where
    T: Stream,
{
    type Output = Option<T::Item>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.project();
        Pin::new(&mut **this.stream).poll_next(cx)
    }
}
