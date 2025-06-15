use rquickjs::{Ctx, FromJs, JsLifetime, Object, atom::PredefinedAtom, class::Trace};
use rquickjs_util::StringRef;
use usql_any::AnyRow;
use usql_core::{ColumnIndex, Row};
use usql_value::Value;

use crate::value::Val;

#[rquickjs::class]
pub struct JsRow {
    pub inner: AnyRow,
}

impl<'js> Trace<'js> for JsRow {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl JsLifetime<'_> for JsRow {
    type Changed<'to> = JsRow;
}

#[rquickjs::methods]
impl JsRow {
    fn get(&self, column: Column<'_>) -> rquickjs::Result<Val> {
        let value = self.inner.get((&column).into());
        match value {
            Ok(value) => Ok(Val(value.to_owned())),
            Err(_) => Ok(Val(Value::Null)),
        }
    }

    #[qjs(rename = "columnName")]
    fn column_name(&self, idx: usize) -> rquickjs::Result<Option<String>> {
        Ok(self.inner.column_name(idx).map(|m| m.to_string()))
    }

    fn length(&self) -> usize {
        self.inner.len()
    }

    #[qjs(rename = PredefinedAtom::ToJSON)]
    fn to_json<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Object<'js>> {
        let output = Object::new(ctx)?;

        for idx in 0..self.inner.len() {
            let name = self.inner.column_name(idx).expect("column name");
            let value = self.inner.get(ColumnIndex::Index(idx)).expect("column");
            output.set(name, Val(value.to_owned()))?;
        }

        Ok(output)
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
