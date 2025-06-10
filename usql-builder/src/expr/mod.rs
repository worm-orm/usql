use crate::{context::Context, error::Error};

mod binary;
mod call;
mod ext;
mod ident;
mod value;

pub use self::{binary::*, call::*, ext::*, ident::Ident, value::val};

pub trait Expression<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;

    fn is_null(&self) -> bool {
        false
    }
}

impl<'a, 'b> Expression<'a> for &'b str {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self)
    }
}

// pub trait IntoExpression<'a> {
//     type Expression: Expression<'a>;

//     fn into_expression(self) -> Self::Expression;
// }
