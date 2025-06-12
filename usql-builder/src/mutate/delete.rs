use core::fmt::Write;

use alloc::borrow::Cow;

use crate::{
    expr::{Expression, ExpressionBox, expr_box},
    statement::Statement,
};

pub struct Delete<'a> {
    table: Cow<'a, str>,
    filter: Option<ExpressionBox<'a>>,
}

impl<'a> Delete<'a> {
    pub fn new(table: impl Into<Cow<'a, str>>) -> Delete<'a> {
        Delete {
            table: table.into(),
            filter: None,
        }
    }

    pub fn filter<E>(mut self, filter: E) -> Self
    where
        E: Expression<'a> + Send + Sync + Clone + 'a,
    {
        self.filter = Some(expr_box(filter));
        self
    }
}

impl<'a> Statement<'a> for Delete<'a> {
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        ctx.write_str("DELETE FROM ")?;
        ctx.push_identifier(&self.table)?;

        if let Some(filter) = self.filter {
            ctx.write_str(" WHERE ")?;
            filter.build(ctx)?;
        }

        Ok(())
    }
}
