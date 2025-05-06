use rquickjs::{JsLifetime, class::Trace};
use usql::AnyTransaction;

#[derive(JsLifetime)]
#[rquickjs::class]
pub struct JsTrans {
    pub i: Option<AnyTransaction<'static>>,
}

impl<'js> Trace<'js> for JsTrans {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}
