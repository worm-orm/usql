use std::mem::transmute;

use futures::{StreamExt, TryStreamExt};
use rquickjs::{Class, Ctx, Function, JsLifetime, class::Trace};
use rquickjs_util::{StringRef, throw, throw_if};
use usql::{AnyConn, Connection};

use crate::{row::JsRow, statement::StatementOrQuery, value::Val};

use super::{statement::JsStatement, trans::JsTrans};

#[rquickjs::class]
pub struct JsConn {
    conn: AnyConn,
}

impl<'js> Trace<'js> for JsConn {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl JsLifetime<'_> for JsConn {
    type Changed<'to> = JsConn;
}

#[rquickjs::methods]
impl JsConn {
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
        params: Vec<Val>,
    ) -> rquickjs::Result<Vec<JsRow>> {
        match stmt {
            StatementOrQuery::Query(query) => {
                let mut stmt = throw_if!(ctx, self.conn.prepare(query.as_str()).await);
                let stream = self.conn.query(
                    &mut stmt,
                    params.into_iter().map(|m| m.0).collect::<Vec<_>>(),
                );

                let rows = stream
                    .map_ok(|row| JsRow { inner: row })
                    .try_collect::<Vec<_>>()
                    .await;

                drop(stmt);

                let rows = throw_if!(ctx, rows);

                Ok(rows)
            }
            StatementOrQuery::Statement(stmt) => {
                let mut guard = stmt.borrow_mut();

                let Some(stmt) = &mut guard.inner else {
                    throw!(ctx, "Statement is finalized")
                };

                let stream = self
                    .conn
                    .query(stmt, params.into_iter().map(|m| m.0).collect::<Vec<_>>());

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

    async fn transaction<'js>(
        &mut self,
        ctx: Ctx<'js>,
        cb: Function<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        let trans = throw_if!(ctx, self.conn.begin().await);

        let trans = Class::instance(
            ctx,
            JsTrans {
                // Safety: We are only using the transaction for the duration of this call
                // making sure to release it before return
                // Also the we have mutable exclusive access to jsconn
                i: Some(unsafe { transmute(trans) }),
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

        trans.borrow_mut().i = None;

        ret
    }
}
