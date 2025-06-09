use crate::{context::Context, error::Error};

pub trait Statement<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}
