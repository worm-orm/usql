use crate::{
    context::Context, error::Error, expr::Expression, select::query::Query, statement::Statement,
};
use core::fmt::Write;

pub struct FilterSelect<S, E> {
    select: S,
    expression: E,
}

impl<S, E> FilterSelect<S, E> {
    pub fn new(select: S, expression: E) -> FilterSelect<S, E> {
        FilterSelect { select, expression }
    }
}

impl<'a, S, E> Query<'a> for FilterSelect<S, E>
where
    S: Query<'a>,
    E: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.select.build(ctx)?;
        ctx.write_str(" WHERE ")?;
        self.expression.build(ctx)?;
        Ok(())
    }
}

impl<'a, S, E> Statement<'a> for FilterSelect<S, E>
where
    S: Query<'a>,
    E: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Query<'a>>::build(self, ctx)
    }
}
