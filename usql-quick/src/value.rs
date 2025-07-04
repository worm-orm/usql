use std::collections::BTreeMap;

use rquickjs::{
    Array, FromJs, IntoJs, IteratorJs, String as JsString, Type, Value as JsValue, class::Trace,
};

use rquickjs_util::{Map as JsMap, Set as JsSet, date::Date};
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

macro_rules! un {
    ($expr: expr) => {
        $expr.map_err(|v| rquickjs::Error::new_from_js(v.type_name(), "value"))
    };
}

fn from_js<'js>(
    ctx: &rquickjs::prelude::Ctx<'js>,
    value: rquickjs::Value<'js>,
) -> rquickjs::Result<Val> {
    match value.type_of() {
        Type::Bool => Ok(Val(Value::Bool(value.as_bool().unwrap()))),
        Type::String => Ok(Val(Value::Text(un!(value.try_into_string())?.to_string()?))),
        Type::Int => Ok(Val(Value::BigInt(value.as_int().unwrap().into()))),
        Type::Float => Ok(Val(Value::Double(value.as_float().unwrap().into()))),
        Type::Null | Type::Undefined => Ok(Val(Value::Null)),
        Type::Array => {
            let array = un!(value.try_into_array())?;
            Ok(Val(Value::Array(
                array
                    .iter::<Val>()
                    .map(|m| m.map(|m| m.0))
                    .collect::<Result<_, _>>()?,
            )))
        }
        Type::Object => {
            if Date::is(ctx, &value)? {
                let date = Date::from_js(ctx, value)?;

                let chrono_date = date.to_datetime()?;

                Ok(Val(Value::Timestamp(chrono_date.naive_utc())))
            } else if JsMap::is(ctx, &value)? {
                let m = JsMap::from_js(ctx, value)?;
                let mut map = BTreeMap::default();
                for next in m.entries::<String, Val>()? {
                    let (k, v) = next?;
                    map.insert(k, v.0);
                }

                todo!("map")

                //Ok(Val(Value::Json(map.into())))
            } else if JsSet::is(ctx, &value)? {
                let m = JsSet::from_js(ctx, value)?;
                let mut list = Vec::default();
                for next in m.entries::<Val>(ctx.clone())? {
                    let (_, v) = next?;
                    list.push(v.0);
                }

                Ok(Val(Value::Array(list)))
            } else {
                // let object = un!(value.try_into_object())?;

                // let mut map = Map::default();
                // for k in object.keys::<String>() {
                //     let k = k?;
                //     let v = object.get::<_, Val>(&k)?;
                //     map.insert(k, v.0);
                // }

                // Ok(Val(Value::Map(map)))
                todo!("map")
            }
        }
        Type::Exception => {
            let exption = un!(value.try_into_exception())?;
            Ok(Val(Value::Text(exption.to_string())))
        }
        _ => Err(rquickjs::Error::new_from_js("value", "value")),
    }
}

impl<'js> FromJs<'js> for Val {
    fn from_js(
        ctx: &rquickjs::prelude::Ctx<'js>,
        value: rquickjs::Value<'js>,
    ) -> rquickjs::Result<Self> {
        from_js(ctx, value)
    }
}

impl<'js> IntoJs<'js> for Val {
    fn into_js(self, ctx: &rquickjs::prelude::Ctx<'js>) -> rquickjs::Result<JsValue<'js>> {
        let val = match self.0 {
            Value::Bool(b) => JsValue::new_bool(ctx.clone(), b),
            Value::Text(t) => JsString::from_str(ctx.clone(), t.as_str())?.into(),
            Value::Json(_map) => {
                // let obj = rquickjs::Object::new(ctx.clone())?;
                // for (k, v) in map {
                //     obj.set(k.as_str(), Val(v).into_js(ctx)?)?;
                // }
                // obj.into_value()
                todo!("json")
            }
            Value::Array(list) => {
                let items = list
                    .into_iter()
                    .map(|value| Val(value).into_js(ctx))
                    .collect_js::<Array>(ctx)?;

                items.into_value()
            }
            Value::ByteArray(bs) => rquickjs::ArrayBuffer::new(ctx.clone(), bs)?.into_value(),
            // Value::Date(_) => todo!(),
            Value::Timestamp(date) => Date::from_chrono(ctx, date.and_utc()).into_js(&ctx)?,
            // Value::Time(_) => todo!(),
            Value::Uuid(b) => {
                JsString::from_str(ctx.clone(), &b.as_hyphenated().to_string())?.into()
            }
            Value::BigInt(n) => JsValue::new_int(ctx.clone(), n as _),
            Value::Int(n) => JsValue::new_int(ctx.clone(), n),
            Value::Null => JsValue::new_null(ctx.clone()),
            _ => return Err(rquickjs::Error::new_into_js("value", "value")),
        };

        Ok(val)
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
