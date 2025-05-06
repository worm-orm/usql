use rquickjs::{Ctx, FromJs, JsLifetime, class::Trace};
use rquickjs_util::StringRef;
use usql::{AnyRow, ColumnIndex, Row, Value};

use crate::value::Val;

#[rquickjs::class]
pub struct JsRow {
    pub inner: AnyRow,
}

impl<'js> Trace<'js> for JsRow {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl JsLifetime<'_> for JsRow {
    type Changed<'to> = JsRow;
}

#[rquickjs::methods]
impl JsRow {
    fn get<'js>(&self, column: Column<'js>) -> rquickjs::Result<Val> {
        let value = self.inner.get((&column).into());
        match value {
            Ok(value) => Ok(Val(value.to_owned())),
            Err(_) => Ok(Val(Value::Null)),
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

pub enum Column<'a> {
    Index(u32),
    Named(StringRef<'a>),
}

impl<'js> FromJs<'js> for Column<'js> {
    fn from_js(ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        if value.is_string() {
            Ok(Column::Named(StringRef::from_js(ctx, value)?))
        } else {
            Ok(Column::Index(u32::from_js(ctx, value)?))
        }
    }
}

impl<'a, 'b> From<&'b Column<'a>> for ColumnIndex<'b> {
    fn from(value: &'b Column<'a>) -> Self {
        match value {
            Column::Index(idx) => ColumnIndex::Index(*idx as _),
            Column::Named(named) => ColumnIndex::Named(named.as_str()),
        }
    }
}
