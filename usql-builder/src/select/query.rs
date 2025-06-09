use crate::{
    context::Context,
    error::Error,
    expr::Expression,
    select::{
        filter::FilterSelect,
        join::{JoinSelect, Joinable},
    },
};

pub trait Query<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

pub trait FilterQuery<'a>: Query<'a> + Sized {
    fn filter<E: Expression<'a>>(self, expression: E) -> FilterSelect<Self, E> {
        FilterSelect::new(self, expression)
    }
}

pub trait JoinQuery<'a>: Query<'a> + Sized {
    fn join<T: Joinable<'a>>(self, joinable: T) -> JoinSelect<Self, T> {
        JoinSelect::new(self, joinable)
    }
}

impl<'a, S, J> JoinQuery<'a> for JoinSelect<S, J>
where
    S: Query<'a>,
    J: Joinable<'a>,
{
}

impl<'a, S, J> FilterQuery<'a> for JoinSelect<S, J>
where
    S: Query<'a>,
    J: Joinable<'a>,
{
}
