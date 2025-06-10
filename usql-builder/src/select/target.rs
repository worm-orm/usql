use core::fmt::Write;

use crate::{
    context::Context,
    error::Error,
    expr::{Expression, Ident},
    select::{Alias, Selection},
};

pub trait Target<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

impl<'a, 'b> Target<'a> for &'b str {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.push_identifier(self)
    }
}

macro_rules! target {
    ($first: ident) => {
        impl<'val, $first: Target<'val>> Target<'val> for ($first,) {
            #[inline]
            fn build(self, ctx: &mut Context<'val>) -> Result<(),Error> {
                <$first as Target>::build(self.0, ctx)?;
                Ok(())
            }
        }

    };
    ($first:ident $( $rest: ident )*) => {
        target!($($rest)*);

        impl<'val,$first: Target<'val>, $( $rest: Target<'val> ),*> Target<'val> for ($first, $($rest),*) {

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

target!(
    C1 C2 C3 C4 C5 C6 C7 C8 C9 C10 C11 C12
);

// Move those below

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableColumn<T, C> {
    pub table: T,
    pub column: C,
}

impl<T, C> TableColumn<T, C> {
    pub fn new(table: T, column: C) -> TableColumn<T, C> {
        TableColumn { table, column }
    }
}

impl<'a, T, C> Selection<'a> for TableColumn<T, C>
where
    T: Ident<'a>,
    C: Selection<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Ident<'a>>::build(self, ctx)
    }
}

impl<'a, T, C> Ident<'a> for TableColumn<T, C>
where
    T: Ident<'a>,
    C: Selection<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.table.build(ctx)?;
        ctx.write_char('.')?;
        self.column.build(ctx)?;
        Ok(())
    }
}

impl<'a, T, C> Expression<'a> for TableColumn<T, C>
where
    T: Ident<'a>,
    C: Selection<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Ident<'a>>::build(self, ctx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Aliased<T, A> {
    pub target: T,
    pub alias: A,
}

impl<T, A> Aliased<T, A> {
    pub fn new(target: T, alias: A) -> Aliased<T, A> {
        Aliased { target, alias }
    }
}

impl<'a, T, A> Ident<'a> for Aliased<T, A>
where
    A: Alias<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.alias.build(ctx)
    }
}

impl<'a, T, A> Target<'a> for Aliased<T, A>
where
    T: Target<'a>,
    A: Alias<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.target.build(ctx)?;
        ctx.write_str(" AS ")?;
        self.alias.build(ctx)?;

        Ok(())
    }
}

impl<'a, T, A> Selection<'a> for Aliased<T, A>
where
    T: Ident<'a>,
    A: Alias<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.target.build(ctx)?;
        ctx.write_str(" AS ")?;
        self.alias.build(ctx)?;

        Ok(())
    }
}

impl<'a, T, A> Expression<'a> for Aliased<T, A>
where
    T: Ident<'a>,
    A: Alias<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        self.alias.build(ctx)
    }
}

pub fn table<'a, T: Target<'a>>(target: T) -> T {
    target
}

pub fn column<'a, T: Selection<'a>>(target: T) -> T {
    target
}

pub trait TargetExt<'a>: Target<'a> + Sized {
    fn col<C>(self, column: C) -> TableColumn<Self, C>
    where
        C: Selection<'a>,
    {
        TableColumn::new(self, column)
    }

    // fn alias<A>(self, alias: A) -> Aliased<Self, A>
    // where
    //     A: Alias<'a>,
    // {
    //     Aliased::new(self, alias)
    // }
}

impl<'a, T> TargetExt<'a> for T where T: Target<'a> {}

pub trait IdentExt<'a>: Ident<'a> + Sized {
    fn alias<A>(self, alias: A) -> Aliased<Self, A>
    where
        A: Alias<'a>,
    {
        Aliased::new(self, alias)
    }
}

impl<'a, T> IdentExt<'a> for T where T: Ident<'a> {}
