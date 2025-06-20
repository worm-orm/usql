use crate::{
    context::Context,
    error::Error,
    expr::{BinaryExpression, BinaryOperator, Expression},
    select::query::Query,
};
use core::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FilterSelect<S, E> {
    select: S,
    expression: E,
}

impl<S, E> FilterSelect<S, E> {
    pub fn new(select: S, expression: E) -> FilterSelect<S, E> {
        FilterSelect { select, expression }
    }

    pub fn and<O>(self, expr: O) -> FilterSelect<S, BinaryExpression<E, O>> {
        FilterSelect {
            select: self.select,
            expression: BinaryExpression::new(self.expression, expr, BinaryOperator::And),
        }
    }

    pub fn or<O>(self, expr: O) -> FilterSelect<S, BinaryExpression<E, O>> {
        FilterSelect {
            select: self.select,
            expression: BinaryExpression::new(self.expression, expr, BinaryOperator::Or),
        }
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
