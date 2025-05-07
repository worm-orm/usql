use futures::TryStreamExt;
use rquickjs::{Class, Ctx, FromJs, JsLifetime, class::Trace};
use rquickjs_util::{StringRef, throw, throw_if};
use usql::{AnyConnector, AnyStatement, Executor};

use crate::{JsRow, Val};

#[rquickjs::class(rename = "Statement")]
pub struct JsStatement {
    pub inner: Option<AnyStatement>,
}

impl<'js> Trace<'js> for JsStatement {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl JsLifetime<'_> for JsStatement {
    type Changed<'to> = JsStatement;
}

#[rquickjs::methods]
impl JsStatement {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>) -> rquickjs::Result<JsStatement> {
        throw!(ctx, "Statement cannot be constructed directly")
    }

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

impl<'js> StatementOrQuery<'js> {
    pub async fn query<T: Executor<Connector = AnyConnector>>(
        &self,
        ctx: Ctx<'js>,
        executor: &T,
        params: Vec<Val>,
    ) -> rquickjs::Result<Vec<JsRow>> {
        match self {
            StatementOrQuery::Query(query) => {
                let mut stmt = throw_if!(ctx, executor.prepare(query.as_str()).await);
                let stream = executor.query(
                    &mut stmt,
                    params.into_iter().map(|m| m.0).collect::<Vec<_>>(),
                );

                let rows = stream
                    .map_ok(|row| JsRow { inner: row })
                    .try_collect::<Vec<_>>()
                    .await;

                let rows = throw_if!(ctx, rows);

                Ok(rows)
            }
            StatementOrQuery::Statement(stmt) => {
                let mut guard = stmt.borrow_mut();

                let Some(stmt) = &mut guard.inner else {
                    throw!(ctx, "Statement is finalized")
                };

                let stream =
                    executor.query(stmt, params.into_iter().map(|m| m.0).collect::<Vec<_>>());

                let rows = stream
                    .map_ok(|row| JsRow { inner: row })
                    .try_collect::<Vec<_>>()
                    .await;

                drop(guard);
                let rows = throw_if!(ctx, rows);

                Ok(rows)
            }
        }
    }

    pub async fn exec<T: Executor<Connector = AnyConnector>>(
        &self,
        ctx: Ctx<'js>,
        executor: &T,
        params: Vec<Val>,
    ) -> rquickjs::Result<()> {
        match self {
            StatementOrQuery::Query(query) => {
                let mut stmt = throw_if!(ctx, executor.prepare(query.as_str()).await);
                let ret = executor
                    .exec(
                        &mut stmt,
                        params.into_iter().map(|m| m.0).collect::<Vec<_>>(),
                    )
                    .await;

                throw_if!(ctx, ret);

                Ok(())
            }
            StatementOrQuery::Statement(stmt) => {
                let mut guard = stmt.borrow_mut();

                let Some(stmt) = &mut guard.inner else {
                    throw!(ctx, "Statement is finalized")
                };

                let ret = executor
                    .exec(stmt, params.into_iter().map(|m| m.0).collect::<Vec<_>>())
                    .await;

                drop(guard);
                throw_if!(ctx, ret);

                Ok(())
            }
        }
    }
}
