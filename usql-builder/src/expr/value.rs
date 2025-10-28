use usql_value::{Value, ValueCow, ValueRef};

use crate::expr::Expression;

impl<'a> Expression<'a> for ValueRef<'a> {
    fn build(self, ctx: &mut crate::context::Context<'a>) -> Result<(), crate::error::Error> {
        ctx.push(self)
    }

    fn is_null(&self) -> bool {
        matches!(self, ValueRef::Null)
    }
}

impl<'a> Expression<'a> for Value {
    fn build(self, ctx: &mut crate::context::Context<'a>) -> Result<(), crate::error::Error> {
        ctx.push(self)
    }

    fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl<'a> Expression<'a> for &'a Value {
    fn build(self, ctx: &mut crate::context::Context<'a>) -> Result<(), crate::error::Error> {
        ctx.push(self.as_ref())
    }

    fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

impl<'a> Expression<'a> for ValueCow<'a> {
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        ctx.push(self)
    }

    fn is_null(&self) -> bool {
        matches!(self.as_ref(), ValueRef::Null)
    }
}

pub fn val<T: Into<Value>>(value: T) -> Value {
    value.into()
}
