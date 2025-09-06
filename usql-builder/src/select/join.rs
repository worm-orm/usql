use crate::{
    context::Context,
    either::Either,
    error::Error,
    expr::Expression,
    select::{join, query::Query},
};

use super::target::Target;

use alloc::{boxed::Box, vec::Vec};
use core::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct JoinSelect<S, J> {
    select: S,
    joinable: J,
}

impl<S, J> JoinSelect<S, J> {
    pub fn new(select: S, joinable: J) -> JoinSelect<S, J> {
        JoinSelect { select, joinable }
    }
}

impl<'a, S, J> Query<'a> for JoinSelect<S, J>
where
    S: Query<'a>,
    J: Joinable<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.select.build(ctx)?;
        ctx.write_str(" ")?;
        self.joinable.build(ctx)?;

        Ok(())
    }
}

pub trait Joinable<'val> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error>;
}

impl<'val, V> Joinable<'val> for Vec<V>
where
    V: Joinable<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        for (idx, v) in self.into_iter().enumerate() {
            if idx > 0 {
                ctx.write_char(' ')?;
            }
            v.build(ctx)?;
        }

        Ok(())
    }
}

impl<'val, L, R> Joinable<'val> for Either<L, R>
where
    L: Joinable<'val>,
    R: Joinable<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        match self {
            Either::Left(e) => e.build(ctx),
            Either::Right(e) => e.build(ctx),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Join<T> {
    kind: JoinType,
    table: T,
}

impl<'val, T> Joinable<'val> for Join<T>
where
    T: Target<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        self.kind.build(ctx)?;
        ctx.write_str(" ")?;
        <T as Target>::build(self.table, ctx)?;
        Ok(())
    }
}

impl<T> Join<T> {
    pub fn inner(table: T) -> Join<T> {
        Join {
            kind: JoinType::Inner,
            table,
        }
    }

    pub fn left(table: T) -> Join<T> {
        Join {
            kind: JoinType::Left,
            table,
        }
    }
    pub fn on<E>(self, e: E) -> JoinOn<T, E> {
        JoinOn { join: self, on: e }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct JoinOn<T, E> {
    join: Join<T>,
    on: E,
}

impl<'val, T, E> Joinable<'val> for JoinOn<T, E>
where
    E: Expression<'val>,
    T: Target<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        self.join.build(ctx)?;
        ctx.write_str(" ON ")?;
        self.on.build(ctx)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Outer,
}

impl JoinType {
    fn build(&self, ctx: &mut Context<'_>) -> Result<(), Error> {
        match self {
            JoinType::Inner => ctx.write_str("INNER JOIN"),
            JoinType::Left => ctx.write_str("LEFT JOIN"),
            JoinType::Right => ctx.write_str("RIGHT JOIN"),
            JoinType::Outer => ctx.write_str("OUTER JOIN"),
        }?;
        Ok(())
    }
}

pub type BoxJoinable<'val> = Box<dyn DynJoinable<'val> + 'val>;
pub trait DynJoinable<'val> {
    fn build(self: Box<Self>, ctx: &mut Context<'val>) -> Result<(), Error>;
}

impl<'val> DynJoinable<'val> for Box<dyn DynJoinable<'val>> {
    fn build(self: Box<Self>, ctx: &mut Context<'val>) -> Result<(), Error> {
        (*self).build(ctx)
    }
}

impl<'val> Joinable<'val> for Box<dyn DynJoinable<'val>> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        self.build(ctx)
    }
}

struct JoinableBox<T>(T);

impl<'val, T> DynJoinable<'val> for JoinableBox<T>
where
    T: Joinable<'val>,
{
    fn build(self: Box<Self>, ctx: &mut Context<'val>) -> Result<(), Error> {
        self.0.build(ctx)
    }
}

pub fn joinable_box<'val, T>(joinable: T) -> Box<dyn DynJoinable<'val> + 'val>
where
    T: Joinable<'val> + 'val,
{
    Box::new(JoinableBox(joinable))
}
