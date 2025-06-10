use crate::{
    Context, Error,
    expr::{Expression, Ident},
};
use alloc::vec::Vec;
use core::fmt::{Display, Write};

pub fn call<T, A>(func: T, args: A) -> Call<T, A> {
    Call::new(func, args)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Call<T, A> {
    func: T,
    args: A,
}

impl<T, A> Call<T, A> {
    pub fn new(func: T, args: A) -> Call<T, A> {
        Call { func, args }
    }
}

impl<'a, T, A> Expression<'a> for Call<T, A>
where
    T: Display,
    A: Args<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        write!(ctx, "{}", self.func)?;
        ctx.write_char('(')?;
        self.args.build(ctx, ",")?;
        ctx.write_char(')')?;
        Ok(())
    }
}

impl<'a, T, A> Ident<'a> for Call<T, A>
where
    T: Display,
    A: Args<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Expression<'a>>::build(self, ctx)
    }
}

pub trait Args<'val> {
    fn build(self, ctx: &mut Context<'val>, sep: &str) -> Result<(), Error>;
}

impl<'val> Args<'val> for () {
    fn build(self, _ctx: &mut Context<'val>, _sep: &str) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! args_impl {
  ($first: ident) => {
      impl<'val, $first: Expression<'val>> Args<'val> for ($first,)  {
          fn build(self, ctx: &mut Context<'val>, _sep: &str) -> Result<(), Error> {
              <$first as Expression<'val>>::build(self.0, ctx)
          }
      }
  };
  ($first: ident $($rest: ident)*) => {
      args_impl!($($rest)*);

      impl<'val, $first: Expression<'val>, $($rest: Expression<'val>),*> Args<'val> for ($first, $($rest),*)  {
          #[allow(non_snake_case)]
          fn build(self, ctx: &mut Context<'val>, sep:&str) -> Result<(), Error> {
              let ($first, $($rest),*) = self;
              <$first as Expression<'val>>::build($first, ctx)?;
              $(
                  ctx.write_str(sep)?;
                  <$rest as Expression<'val>>::build($rest, ctx)?;
              )*

              Ok(())
          }
      }
  }
}

impl<'a, T> Args<'a> for Vec<T>
where
    T: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>, sep: &str) -> Result<(), Error> {
        for (i, n) in self.into_iter().enumerate() {
            if i > 0 {
                ctx.write_str(sep)?;
            }
            <T as Expression>::build(n, ctx)?;
        }

        Ok(())
    }
}

args_impl!(T1 T2 T3 T4 T5 T6 T7 T8);
