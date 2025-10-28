use klaver_util::throw_if;
use rquickjs::{FromJs, IntoJs, Value as JsValue, class::Trace};
use usql_value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Val(pub usql_value::Value);

impl core::ops::Deref for Val {
    type Target = usql_value::Value;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for Val {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'js> Trace<'js> for Val {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

impl<'js> FromJs<'js> for Val {
    fn from_js(
        ctx: &rquickjs::prelude::Ctx<'js>,
        value: rquickjs::Value<'js>,
    ) -> rquickjs::Result<Self> {
        let value = bycat_value::Value::from_js(ctx, value)?;
        let value: Value = throw_if!(ctx, value.try_into());

        Ok(Val(value))
    }
}

impl<'js> IntoJs<'js> for Val {
    fn into_js(self, ctx: &rquickjs::prelude::Ctx<'js>) -> rquickjs::Result<JsValue<'js>> {
        let value: bycat_value::Value = throw_if!(ctx, self.0.try_into());
        value.into_js(ctx)
    }
}

#[cfg(test)]
mod test {
    use rquickjs::{Context, Runtime};

    use super::Val;

    #[test]
    fn test_val() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();

        let _value = context
            .with(|ctx| {
                //
                let val = ctx.eval::<Val, _>("({name: new Date(), age: 20})")?;

                rquickjs::Result::Ok(val.0)
            })
            .unwrap();
    }
}
