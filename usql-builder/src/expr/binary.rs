use core::fmt::Write;

use crate::{context::Context, error::Error, expr::Expression};

#[derive(Debug, Clone, PartialEq, Eq, Copy, PartialOrd, Ord, Hash)]
pub enum BinaryOperator {
    And,
    Or,
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    NotEq,
    In,
    Like,
    NoLike,
    Div,
    Sub,
    Mul,
    Add,
    Match,
    ExtractText, // ->
    Extract,     // ->>
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct BinaryExpression<L, R> {
    pub(crate) operator: BinaryOperator,
    pub(crate) left: L,
    pub(crate) right: R,
}

impl<L, R> BinaryExpression<L, R> {
    pub fn new(left: L, right: R, operator: BinaryOperator) -> Self {
        BinaryExpression {
            left,
            right,
            operator,
        }
    }

    pub fn and<'a, E: Expression<'a>>(self, expr: E) -> BinaryExpression<Self, E> {
        BinaryExpression::new(self, expr, BinaryOperator::And)
    }

    pub fn or<'a, E: Expression<'a>>(self, expr: E) -> BinaryExpression<Self, E> {
        BinaryExpression::new(self, expr, BinaryOperator::Or)
    }
}

impl<'a, L, R> Expression<'a> for BinaryExpression<L, R>
where
    L: Expression<'a>,
    R: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.left.build(ctx)?;
        ctx.write_str(" ")?;
        if self.operator == BinaryOperator::Eq && self.right.is_null() {
            ctx.write_str("IS NULL")?;
        } else if self.operator == BinaryOperator::Eq && self.right.is_null() {
            ctx.write_str("IS NOT NULL")?;
        } else {
            build_binary_operator(ctx, self.operator)?;
            ctx.write_str(" ")?;
            self.right.build(ctx)?;
        }

        Ok(())
    }
}

fn build_binary_operator(ctx: &mut Context<'_>, operator: BinaryOperator) -> Result<(), Error> {
    match operator {
        BinaryOperator::Eq => ctx.write_str("="),
        BinaryOperator::Lt => ctx.write_str("<"),
        BinaryOperator::Lte => ctx.write_str("<="),
        BinaryOperator::Gt => ctx.write_str(">"),
        BinaryOperator::Gte => ctx.write_str(">="),
        BinaryOperator::NotEq => ctx.write_str("!="),
        BinaryOperator::And => ctx.write_str("AND"),
        BinaryOperator::Or => ctx.write_str("OR"),
        BinaryOperator::Like => ctx.write_str("LIKE"),
        BinaryOperator::NoLike => ctx.write_str("NOT LIKE"),
        BinaryOperator::In => ctx.write_str("IN"),
        BinaryOperator::Add => ctx.write_char('+'),
        BinaryOperator::Sub => ctx.write_char('-'),
        BinaryOperator::Div => ctx.write_char('/'),
        BinaryOperator::Mul => ctx.write_char('*'),
        BinaryOperator::Extract => ctx.write_str("->>"),
        BinaryOperator::ExtractText => ctx.write_str("->"),
        BinaryOperator::Match => ctx.write_str("MATCH"),
    }?;
    Ok(())
}
