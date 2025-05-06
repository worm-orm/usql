use rquickjs::{Class, FromJs, JsLifetime, class::Trace};
use rquickjs_util::StringRef;
use usql::AnyStatement;

#[rquickjs::class]
pub struct JsStatement {
    pub inner: Option<AnyStatement>,
}

impl<'js> Trace<'js> for JsStatement {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl JsLifetime<'_> for JsStatement {
    type Changed<'to> = JsStatement;
}

#[rquickjs::methods]
impl JsStatement {
    fn finalize(&mut self) -> rquickjs::Result<()> {
        self.inner = None;
        Ok(())
    }
}

pub enum StatementOrQuery<'js> {
    Statement(Class<'js, JsStatement>),
    Query(StringRef<'js>),
}

impl<'js> FromJs<'js> for StatementOrQuery<'js> {
    fn from_js(ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        if value.is_string() {
            let query = StringRef::from_js(ctx, value)?;
            Ok(StatementOrQuery::Query(query))
        } else {
            let stmt = Class::<JsStatement>::from_js(ctx, value)?;
            Ok(StatementOrQuery::Statement(stmt))
        }
    }
}
