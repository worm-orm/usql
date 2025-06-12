use usql_core::System;

use crate::{context::Context, error::Error, sql::SqlStmt};

pub trait Statement<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

pub trait StatementExt<'a>: Statement<'a> {
    fn to_sql(self, dialect: System) -> Result<SqlStmt<'a>, Error>
    where
        Self: Sized,
    {
        let mut ctx = Context::new(dialect);
        self.build(&mut ctx)?;
        Ok(ctx.build())
    }
}

impl<'a, T> StatementExt<'a> for T where T: Statement<'a> {}
