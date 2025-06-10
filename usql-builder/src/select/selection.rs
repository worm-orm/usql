use core::fmt::Write;

use alloc::vec::Vec;

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

impl<'val, V> Selection<'val> for Vec<V>
where
    V: Selection<'val>,
{
    #[inline]
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        for (idx, col) in self.into_iter().enumerate() {
            if idx != 0 {
                ctx.write_char(',')?;
            }
            col.build(ctx)?;
        }
        Ok(())
    }
}

impl<'a, V> Selection<'a> for &'a [V]
where
    V: Selection<'a> + Clone,
{
    #[inline]
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        for (idx, col) in self.into_iter().enumerate() {
            if idx != 0 {
                ctx.write_char(',')?;
            }
            col.clone().build(ctx)?;
        }
        Ok(())
    }
}

macro_rules! selection {
    ($first: ident) => {
        impl<'val, $first: Selection<'val>> Selection<'val> for ($first,) {
            #[inline]
            fn build(self, ctx: &mut Context<'val>) -> Result<(),Error> {
                <$first as Selection>::build(self.0, ctx)?;
                Ok(())
            }
        }

    };
    ($first:ident $( $rest: ident )*) => {
        selection!($($rest)*);

        impl<'val,$first: Selection<'val>, $( $rest: Selection<'val> ),*> Selection<'val> for ($first, $($rest),*) {

            #[allow(non_snake_case)]
            #[inline]
            fn build( self, ctx: &mut Context<'val>) -> Result<(),Error> {
                let ($first, $($rest),*) = self;
                $first.build(ctx)?;
                $(
                    ctx.write_str(", ")?;
                    $rest.build(ctx)?;
                )*
                Ok(())
            }
        }
    };
}

selection!(
    C1 C2 C3 C4 C5 C6 C7 C8 C9 C10 C11 C12
);

pub struct Star;

impl<'val> Selection<'val> for Star {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        ctx.write_char('*')?;
        Ok(())
    }
}
