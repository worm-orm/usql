use core::fmt::Write;

use crate::{expr::Expression, select::Query};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HavingSelect<S, E> {
    select: S,
    expression: E,
}

impl<S, E> HavingSelect<S, E> {
    pub fn new(select: S, expression: E) -> HavingSelect<S, E> {
        HavingSelect { select, expression }
    }
}

impl<'a, S, E> Query<'a> for HavingSelect<S, E>
where
    S: Query<'a>,
    E: Expression<'a>,
{
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        self.select.build(ctx)?;
        ctx.write_str(" HAVING ")?;

        self.expression.build(ctx)?;

        Ok(())
    }
}
