use crate::{context::Context, either::Either, error::Error};

pub trait Selection<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

impl<'a, 'b> Selection<'a> for &'b str {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self)
    }
}

impl<'a, L, R> Selection<'a> for Either<L, R>
where
    L: Selection<'a>,
    R: Selection<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        match self {
            Either::Left(left) => left.build(ctx),
            Either::Right(right) => right.build(ctx),
        }
    }
}
