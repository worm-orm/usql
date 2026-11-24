use alloc::borrow::Cow;
use core::fmt::Write;

use crate::{Context, Error, expr::Expression};

/// SQL that is written as it is.
///
/// Must never contain input from a user, use [`val`](super::val) for that.
pub fn raw<'a>(sql: impl Into<Cow<'a, str>>) -> Raw<'a> {
    Raw(sql.into())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Raw<'a>(Cow<'a, str>);

impl<'a> Expression<'a> for Raw<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str(&self.0)?;
        Ok(())
    }
}
