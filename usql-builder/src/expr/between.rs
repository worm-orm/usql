use crate::expr::Expression;
use core::fmt::Write;

pub struct Between<T, L, R> {
    expr: T,
    left: L,
    right: R,
}

impl<T, L, R> Between<T, L, R> {
    pub fn new(expr: T, left: L, right: R) -> Between<T, L, R> {
        Between { expr, left, right }
    }
}

impl<'a, T, L, R> Expression<'a> for Between<T, L, R>
where
    T: Expression<'a>,
    L: Expression<'a>,
    R: Expression<'a>,
{
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        self.expr.build(ctx)?;
        write!(ctx, " BETWEEN ")?;
        self.left.build(ctx)?;
        write!(ctx, " AND ")?;
        self.right.build(ctx)?;
        Ok(())
    }
}
