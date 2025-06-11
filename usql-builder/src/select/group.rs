use core::fmt::Write;

use crate::{
    expr::Expression,
    select::{HavingSelect, Query},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupSelect<S, G> {
    select: S,
    group: G,
}

impl<S, G> GroupSelect<S, G> {
    pub fn new(select: S, group: G) -> GroupSelect<S, G> {
        GroupSelect { select, group }
    }

    pub fn having<E>(self, expression: E) -> HavingSelect<Self, E> {
        HavingSelect::new(self, expression)
    }
}

impl<'a, S, G> Query<'a> for GroupSelect<S, G>
where
    S: Query<'a>,
    G: Expression<'a>,
{
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        self.select.build(ctx)?;
        ctx.write_str(" GROUP BY ")?;
        self.group.build(ctx)?;

        Ok(())
    }
}
