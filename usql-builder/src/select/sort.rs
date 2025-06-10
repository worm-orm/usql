use crate::{Context, Error, expr::Ident, select::Query};

use alloc::vec::Vec;
use core::fmt::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortSelect<S, O> {
    select: S,
    order: O,
}

impl<S, O> SortSelect<S, O> {
    pub fn new(select: S, order: O) -> SortSelect<S, O> {
        SortSelect { select, order }
    }
}

impl<'a, S, O> Query<'a> for SortSelect<S, O>
where
    S: Query<'a>,
    O: SortKey<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.select.build(ctx)?;
        write!(ctx, " ORDER BY ")?;
        self.order.build(ctx)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Order {
    Asc,
    Desc,
}

impl core::fmt::Display for Order {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Order::Asc => write!(f, "ASC"),
            Order::Desc => write!(f, "DESC"),
        }
    }
}

pub trait SortKey<'val> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error>;
}

impl<'val, C> SortKey<'val> for (C, Order)
where
    C: Ident<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <C as Ident<'_>>::build(self.0, ctx)?;
        write!(ctx, " {}", self.1)?;
        Ok(())
    }
}

macro_rules! sort_key {
    ($first: ident) => {
        impl<'val, $first: SortKey<'val>> SortKey<'val> for ($first,) {
            #[inline]
            fn build(self, ctx: &mut Context<'val>) -> Result<(),Error> {
                <$first as SortKey>::build(self.0, ctx)?;
                Ok(())
            }
        }

    };
    ($first: ident, $( $rest:ident ),*) => {
        sort_key!($($rest),*);

        impl<'val, $first: SortKey<'val>, $( $rest: SortKey<'val> ),*> SortKey<'val> for ($first, $($rest),*) {

            #[inline]
            #[allow(non_snake_case)]
            fn build(self, ctx: &mut Context<'val>) -> Result<(),Error> {

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

sort_key!(T1, T2, T3, T4, T5, T6, T7, T8);

impl<'val, S: SortKey<'val>> SortKey<'val> for Vec<S> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        for (c, v) in self.into_iter().enumerate() {
            if c > 0 {
                ctx.write_char(',')?;
            }

            v.build(ctx)?
        }
        Ok(())
    }
}
