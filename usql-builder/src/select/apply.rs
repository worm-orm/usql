use crate::{Either, select::Query};

pub trait Apply<'a, T>
where
    T: Query<'a>,
{
    type Output: Query<'a>;

    fn apply(self, query: T) -> Self::Output;
}

impl<'a, T, V> Apply<'a, T> for Option<V>
where
    T: Query<'a>,
    V: Apply<'a, T>,
{
    type Output = Either<T, V::Output>;

    fn apply(self, query: T) -> Self::Output {
        match self {
            Some(m) => Either::Right(m.apply(query)),
            None => Either::Left(query),
        }
    }
}
