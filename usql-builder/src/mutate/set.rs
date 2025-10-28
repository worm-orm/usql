use core::fmt::Write;

use alloc::borrow::Cow;

use crate::{Context, Either, Error, expr::Expression, select::Selection, statement::Statement};

pub trait Set<'key, 'val> {
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'key, str>>,
        V: Expression<'val> + Send + Sync + Clone + 'val;

    fn with<F, V>(mut self, field: F, value: V) -> Self
    where
        Self: Sized,
        F: Into<Cow<'key, str>>,
        V: Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.set(field, value);
        self
    }

    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error>;
}

#[derive(Debug, Clone, Copy)]
pub struct ReturningStmt<S, T> {
    selection: S,
    stmt: T,
}

impl<'val, S, T> Statement<'val> for ReturningStmt<S, T>
where
    S: Selection<'val>,
    T: Statement<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        self.stmt.build(ctx)?;

        write!(ctx, " RETURNING ")?;

        self.selection.build(ctx)?;

        Ok(())
    }
}

pub trait Returning<'val>: Sized {
    fn returning<S>(self, selection: S) -> ReturningStmt<S, Self>
    where
        S: Selection<'val>,
    {
        ReturningStmt {
            selection,
            stmt: self,
        }
    }
}

impl<'val, L, R> Returning<'val> for Either<L, R>
where
    L: Statement<'val>,
    R: Statement<'val>,
{
}
