use core::marker::PhantomData;

use alloc::boxed::Box;
use dyn_clone::DynClone;

use crate::{context::Context, error::Error};

mod binary;
mod call;
mod case;
mod ext;
mod ident;
mod value;

pub use self::{binary::*, call::*, case::*, ext::*, ident::Ident, value::val};

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

#[derive(Debug, Clone)]
struct BoxedExpr<'a, T> {
    expr: T,
    lifetime: PhantomData<&'a ()>,
}

impl<'a, T> ExprBox<'a> for BoxedExpr<'a, T>
where
    T: Expression<'a> + Clone,
{
    fn build(self: Box<Self>, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.expr.build(ctx)
    }
}

pub trait ExprBox<'a>: DynClone {
    fn build(self: Box<Self>, ctx: &mut Context<'a>) -> Result<(), Error>;
}

impl<'a> Expression<'a> for Box<dyn ExprBox<'a> + 'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.build(ctx)
    }
}

impl<'a> Expression<'a> for Box<dyn ExprBox<'a> + Send + Sync + 'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.build(ctx)
    }
}

impl<'a> Clone for ExpressionBox<'a> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

pub type ExpressionBox<'a> = Box<dyn ExprBox<'a> + Send + Sync + 'a>;

pub fn expr_box<'a, E: Expression<'a> + 'a + Send + Sync + Clone>(expr: E) -> ExpressionBox<'a> {
    Box::new(BoxedExpr {
        expr,
        lifetime: PhantomData,
    })
}
