use futures::TryStreamExt;
use rquickjs::{Ctx, JsLifetime, class::Trace};
use rquickjs_util::{StringRef, throw, throw_if};
use usql::{AnyTransaction, prelude::*};

use crate::{JsRow, JsStatement, Val, statement::StatementOrQuery};

#[derive(JsLifetime)]
#[rquickjs::class]
pub struct JsTrans {
    pub i: Option<AnyTransaction<'static>>,
}

impl<'js> Trace<'js> for JsTrans {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

impl JsTrans {
    fn get_conn(&self, ctx: &Ctx<'_>) -> rquickjs::Result<&AnyTransaction<'static>> {
        match &self.i {
            Some(ret) => Ok(ret),
            None => throw!(ctx, "Transaction is closed"),
        }
    }
}

#[rquickjs::methods]
impl JsTrans {
    async fn prepare<'js>(
        &self,
        ctx: Ctx<'js>,
        sql: StringRef<'js>,
    ) -> rquickjs::Result<JsStatement> {
        let stmt = throw_if!(ctx, self.get_conn(&ctx)?.prepare(sql.as_str()).await);
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
                let mut stmt = throw_if!(ctx, self.get_conn(&ctx)?.prepare(query.as_str()).await);
                let stream = self.get_conn(&ctx)?.query(
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

                let stream = self
                    .get_conn(&ctx)?
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
}
