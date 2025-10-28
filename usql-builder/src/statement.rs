use usql_core::System;

use crate::{Either, context::Context, error::Error, sql::SqlStmt};

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

impl<'val, L, R> Statement<'val> for Either<L, R>
where
    L: Statement<'val>,
    R: Statement<'val>,
{
    fn build(self, ctx: &mut crate::Context<'val>) -> Result<(), crate::Error> {
        match self {
            Self::Left(left) => left.build(ctx),
            Self::Right(right) => right.build(ctx),
        }
    }
}
