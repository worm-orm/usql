use core::fmt::Write;

use crate::{
    context::Context,
    error::Error,
    expr::{Expression, Ident},
    select::{
        Selection, SortKey, SortSelect,
        filter::FilterSelect,
        join::{JoinSelect, Joinable},
    },
    statement::Statement,
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

pub trait SortQuery<'a>: Query<'a> + Sized {
    fn order_by<T: SortKey<'a>>(self, order: T) -> SortSelect<Self, T> {
        SortSelect::new(self, order)
    }
}

// Join Select

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

impl<'a, S, J> SortQuery<'a> for JoinSelect<S, J>
where
    S: Query<'a>,
    J: Joinable<'a>,
{
}

// Filter Select

impl<'a, S, E> SortQuery<'a> for FilterSelect<S, E>
where
    S: Query<'a>,
    E: Expression<'a>,
{
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueryStmt<T>(T);

impl<T> QueryStmt<T> {
    pub fn new(query: T) -> QueryStmt<T> {
        QueryStmt(query)
    }
}

impl<'a, T> Statement<'a> for QueryStmt<T>
where
    T: Query<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.0.build(ctx)
    }
}

impl<'a, T> Expression<'a> for QueryStmt<T>
where
    T: Query<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_char('(')?;
        self.0.build(ctx)?;
        ctx.write_char(')')?;
        Ok(())
    }
}

impl<'a, T> Ident<'a> for QueryStmt<T>
where
    T: Query<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Expression<'a>>::build(self, ctx)
    }
}

impl<'a, T> Selection<'a> for QueryStmt<T>
where
    T: Query<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Expression<'a>>::build(self, ctx)
    }
}
