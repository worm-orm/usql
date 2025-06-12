use rquickjs::{Ctx, JsLifetime, class::Trace, prelude::Opt};
use rquickjs_util::{StringRef, throw, throw_if};
use usql_core::{AnyTransaction, prelude::*};

use crate::{JsRow, JsStatement, Val, statement::StatementOrQuery};

#[derive(JsLifetime)]
#[rquickjs::class(rename = "Trans")]
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
        params: Opt<Vec<Val>>,
    ) -> rquickjs::Result<Vec<JsRow>> {
        let conn = self.get_conn(&ctx)?;
        stmt.query(ctx, conn, params.0.unwrap_or_default()).await
    }

    async fn exec<'js>(
        &self,
        ctx: Ctx<'js>,
        stmt: StatementOrQuery<'js>,
        params: Opt<Vec<Val>>,
    ) -> rquickjs::Result<()> {
        let conn = self.get_conn(&ctx)?;
        stmt.exec(ctx, conn, params.0.unwrap_or_default()).await
    }
}
