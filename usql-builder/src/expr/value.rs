use core::fmt::Write;

use alloc::vec::Vec;
use usql_value::{Value, ValueRef};

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

pub fn val<T: Into<Value>>(value: T) -> Value {
    value.into()
}

impl<'a> Expression<'a> for Vec<Value> {
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        ctx.write_char('(')?;
        for (idx, item) in self.into_iter().enumerate() {
            if idx > 0 {
                ctx.write_char(',')?;
            }
            item.build(ctx)?;
        }

        ctx.write_char(')')?;

        Ok(())
    }

    fn is_null(&self) -> bool {
        false
    }
}
