use crate::{
    context::Context,
    error::Error,
    expr::Expression,
    select::{
        query::{FilterQuery, JoinQuery, Query},
        selection::Selection,
        target::Target,
    },
    statement::Statement,
};
use core::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Select<T, S> {
    target: T,
    selection: S,
}

impl<T, S> Select<T, S> {
    pub fn new(target: T, selection: S) -> Select<T, S> {
        Select { target, selection }
    }
}

impl<'a, T, S> Query<'a> for Select<T, S>
where
    T: Target<'a>,
    S: Selection<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        write!(ctx, "SELECT ")?;
        self.selection.build(ctx)?;
        write!(ctx, " FROM ")?;
        self.target.build(ctx)?;
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use usql::System;

    use super::*;
    use crate::{
        context::Context,
        error::Error,
        select::{selection::Selection, target::Target},
    };

    struct MockTarget;
    struct MockSelection;
    struct MockExpression;

    impl<'a> Target<'a> for MockTarget {
        fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
            ctx.write_str("mock_target")?;
            Ok(())
        }
    }

    impl<'a> Selection<'a> for MockSelection {
        fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
            ctx.write_str("mock_selection")?;
            Ok(())
        }
    }

    impl<'a> Expression<'a> for MockExpression {
        fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
            ctx.write_str("mock_expression")?;
            Ok(())
        }
    }

    #[test]
    fn test_select_build() {
        let target = MockTarget;
        let selection = MockSelection;
        let select = Select::new(target, selection);

        let mut ctx = Context::new(System::Sqlite);

        Query::build(select, &mut ctx).expect("build");

        assert_eq!(ctx.to_string(), "SELECT mock_selection FROM mock_target");
    }
}
