use crate::{context::Context, error::Error};

pub trait Target<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

impl<'a, 'b> Target<'a> for &'b str {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self)
    }
}
