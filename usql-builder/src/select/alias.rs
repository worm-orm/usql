use alloc::fmt;

use crate::{context::Context, error::Error};

pub trait Alias<'a>: fmt::Display {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

impl<'a, 'b> Alias<'a> for &'b str {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self)
    }
}
