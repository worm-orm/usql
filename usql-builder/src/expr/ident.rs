use usql_value::Atom;

use crate::{Context, Error};

pub trait Ident<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

impl<'a, 'b> Ident<'a> for &'b str {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self)?;
        Ok(())
    }
}

impl<'a> Ident<'a> for Atom {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self.as_str())?;
        Ok(())
    }
}

impl<'a, 'b> Ident<'a> for &'b Atom {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self.as_str())
    }
}
