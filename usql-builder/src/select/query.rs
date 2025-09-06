use core::fmt::Write;

use alloc::boxed::Box;

use crate::{
    Either,
    context::Context,
    error::Error,
    expr::{Expression, Ident},
    select::{
        HavingSelect, Select, Selection, SortKey, SortSelect, Target,
        apply::Apply,
        filter::FilterSelect,
        group::GroupSelect,
        join::{JoinSelect, Joinable},
        limit::LimitSelect,
    },
    statement::Statement,
};

pub trait Query<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

impl<'a, L, R> Query<'a> for Either<L, R>
where
    L: Query<'a>,
    R: Query<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        match self {
            Self::Left(left) => left.build(ctx),
            Self::Right(right) => right.build(ctx),
        }
    }
}

pub trait FilterQuery<'a>: Query<'a> + Sized {
    fn filter<E: Expression<'a>>(self, expression: E) -> FilterSelect<Self, E> {
        FilterSelect::new(self, expression)
    }
}

impl<'a, L, R> FilterQuery<'a> for Either<L, R>
where
    L: FilterQuery<'a>,
    R: FilterQuery<'a>,
{
}

pub trait JoinQuery<'a>: Query<'a> + Sized {
    fn join<T: Joinable<'a>>(self, joinable: T) -> JoinSelect<Self, T> {
        JoinSelect::new(self, joinable)
    }
}

impl<'a, L, R> JoinQuery<'a> for Either<L, R>
where
    L: JoinQuery<'a>,
    R: JoinQuery<'a>,
{
}

pub trait SortQuery<'a>: Query<'a> + Sized {
    fn order_by<T: SortKey<'a>>(self, order: T) -> SortSelect<Self, T> {
        SortSelect::new(self, order)
    }
}

impl<'a, L, R> SortQuery<'a> for Either<L, R>
where
    L: SortQuery<'a>,
    R: SortQuery<'a>,
{
}

pub trait LimitQuery<'a>: Query<'a> + Sized {
    fn limit(self, offset: u64, limit: u64) -> LimitSelect<Self> {
        LimitSelect::new(self, offset, limit)
    }
}

impl<'a, L, R> LimitQuery<'a> for Either<L, R>
where
    L: LimitQuery<'a>,
    R: LimitQuery<'a>,
{
}

pub trait GroupQuery<'a>: Query<'a> + Sized {
    fn group_by<T>(self, grouping: T) -> GroupSelect<Self, T> {
        GroupSelect::new(self, grouping)
    }
}

impl<'a, L, R> GroupQuery<'a> for Either<L, R>
where
    L: GroupQuery<'a>,
    R: GroupQuery<'a>,
{
}

pub trait QueryExt<'a>: Query<'a> + Sized {
    fn into_stmt(self) -> QueryStmt<Self> {
        QueryStmt::new(self)
    }

    fn apply<T>(self, apply: T) -> T::Output
    where
        T: Apply<'a, Self>,
    {
        apply.apply(self)
    }
}

impl<'a, T> QueryExt<'a> for T where T: Query<'a> {}

// Select

impl<'a, T, S> FilterQuery<'a> for Select<T, S>
where
    T: Target<'a>,
    S: Selection<'a>,
{
}

impl<'a, T, S> JoinQuery<'a> for Select<T, S>
where
    T: Target<'a>,
    S: Selection<'a>,
{
}

impl<'a, T, S> SortQuery<'a> for Select<T, S>
where
    T: Target<'a>,
    S: Selection<'a>,
{
}

impl<'a, T, S> LimitQuery<'a> for Select<T, S>
where
    T: Target<'a>,
    S: Selection<'a>,
{
}

impl<'a, T, S> GroupQuery<'a> for Select<T, S>
where
    T: Target<'a>,
    S: Selection<'a>,
{
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

impl<'a, S, J> LimitQuery<'a> for JoinSelect<S, J>
where
    S: Query<'a>,
    J: Joinable<'a>,
{
}

impl<'a, S, J> GroupQuery<'a> for JoinSelect<S, J>
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

impl<'a, S, E> LimitQuery<'a> for FilterSelect<S, E>
where
    S: Query<'a>,
    E: Expression<'a>,
{
}

impl<'a, S, E> GroupQuery<'a> for FilterSelect<S, E>
where
    S: Query<'a>,
    E: Expression<'a>,
{
}

// Sort select
impl<'a, T, S> LimitQuery<'a> for SortSelect<T, S>
where
    T: Query<'a>,
    S: SortKey<'a>,
{
}

// Group
impl<'a, S, G> LimitQuery<'a> for GroupSelect<S, G>
where
    S: Query<'a>,
    G: Expression<'a>,
{
}

impl<'a, S, G> SortQuery<'a> for GroupSelect<S, G>
where
    S: Query<'a>,
    G: Expression<'a>,
{
}

impl<'a, S, G> LimitQuery<'a> for HavingSelect<S, G>
where
    S: Query<'a>,
    G: Expression<'a>,
{
}

impl<'a, S, G> SortQuery<'a> for HavingSelect<S, G>
where
    S: Query<'a>,
    G: Expression<'a>,
{
}

// Query Statement

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueryStmt<T>(T);

impl<T> QueryStmt<T> {
    pub fn new(query: T) -> QueryStmt<T> {
        QueryStmt(query)
    }

    pub fn into_inner(self) -> T {
        self.0
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

impl<'a, T> Target<'a> for QueryStmt<T>
where
    T: Query<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Expression<'a>>::build(self, ctx)
    }
}

pub trait DynQuery<'a> {
    fn build(self: Box<Self>, ctx: &mut Context<'a>) -> Result<(), Error>;
}

pub struct DynamicQuery<T>(pub T);

impl<T> DynamicQuery<T> {
    pub fn boxed_local<'a>(self) -> Box<dyn DynQuery<'a> + 'a>
    where
        T: Query<'a> + 'a,
    {
        Box::new(self)
    }

    pub fn boxed<'a>(self) -> Box<dyn DynQuery<'a>>
    where
        T: Query<'a> + 'static,
    {
        Box::new(self)
    }
}

impl<'a, T> DynQuery<'a> for DynamicQuery<T>
where
    T: Query<'a>,
{
    fn build(self: Box<Self>, ctx: &mut Context<'a>) -> Result<(), Error> {
        <T as Query<'a>>::build(self.0, ctx)
    }
}

impl<'a> DynQuery<'a> for Box<dyn DynQuery<'a>> {
    fn build(self: Box<Self>, ctx: &mut Context<'a>) -> Result<(), Error> {
        (*self).build(ctx)
    }
}

impl<'a> Query<'a> for Box<dyn DynQuery<'a>> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.build(ctx)
    }
}
