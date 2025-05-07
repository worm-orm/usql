use std::mem::transmute;

use rquickjs::{Class, Ctx, Function, JsLifetime, class::Trace, prelude::Opt};
use rquickjs_util::{StringRef, throw, throw_if};
use usql::{AnyConn, prelude::*};

use crate::{row::JsRow, statement::StatementOrQuery, value::Val};

use super::{statement::JsStatement, trans::JsTrans};

#[rquickjs::class(rename = "Conn")]
pub struct JsConn {
    pub conn: AnyConn,
}

impl<'js> Trace<'js> for JsConn {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl JsLifetime<'_> for JsConn {
    type Changed<'to> = JsConn;
}

#[rquickjs::methods]
impl JsConn {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>) -> rquickjs::Result<JsConn> {
        throw!(ctx, "Conn cannot be constructed directly")
    }

    async fn prepare<'js>(
        &self,
        ctx: Ctx<'js>,
        sql: StringRef<'js>,
    ) -> rquickjs::Result<JsStatement> {
        let stmt = throw_if!(ctx, self.conn.prepare(sql.as_str()).await);
        Ok(JsStatement { inner: Some(stmt) })
    }

    async fn query<'js>(
        &self,
        ctx: Ctx<'js>,
        stmt: StatementOrQuery<'js>,
        params: Opt<Vec<Val>>,
    ) -> rquickjs::Result<Vec<JsRow>> {
        stmt.query(ctx, &self.conn, params.0.unwrap_or_default())
            .await
    }

    async fn exec<'js>(
        &self,
        ctx: Ctx<'js>,
        stmt: StatementOrQuery<'js>,
        params: Opt<Vec<Val>>,
    ) -> rquickjs::Result<()> {
        stmt.exec(ctx, &self.conn, params.0.unwrap_or_default())
            .await
    }

    async fn transaction<'js>(
        &mut self,
        ctx: Ctx<'js>,
        cb: Function<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        let trans = throw_if!(ctx, self.conn.begin().await);

        let trans = Class::instance(
            ctx.clone(),
            JsTrans {
                // Safety: We are only using the transaction for the duration of this call
                // making sure to release it before return
                // Also the we have mutable exclusive access to jsconn
                i: Some(unsafe {
                    transmute::<usql::AnyTransaction<'_>, usql::AnyTransaction<'_>>(trans)
                }),
            },
        )?;

        let ret = match cb.call::<_, rquickjs::Value<'js>>((trans.clone(),)) {
            Ok(ret) => ret,
            Err(err) => {
                trans.borrow_mut().i = None;
                return Err(err);
            }
        };

        let ret = match ret.try_into_promise() {
            Ok(promise) => promise.into_future::<rquickjs::Value<'js>>().await,
            Err(val) => Ok(val),
        };

        let trans = trans.borrow_mut().i.take().expect("transaction");

        if ret.is_err() {
            throw_if!(ctx, trans.rollback().await);
        } else {
            throw_if!(ctx, trans.commit().await);
        }

        // trans.borrow_mut().i = None;

        ret
    }
}
