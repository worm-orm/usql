use core::fmt::Write;

use crate::{
    Context, Error,
    expr::{Expression, Ident},
};

pub fn switch<E, C, T>(expression: E, cases: C, default: T) -> Switch<E, C, T> {
    Switch::new(expression, cases, default)
}

pub fn when<E, T>(condition: E, value: T) -> When<E, T> {
    When::new(condition, value)
}

pub fn ifelse<C, T>(cases: C, default: T) -> IfElse<C, T> {
    IfElse::new(cases, default)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Switch<E, C, T> {
    expression: E,
    cases: C,
    default: T,
}

impl<E, C, T> Switch<E, C, T> {
    pub fn new(expression: E, cases: C, default: T) -> Switch<E, C, T> {
        Switch {
            expression,
            cases,
            default,
        }
    }
}

impl<'a, E, C, T> Expression<'a> for Switch<E, C, T>
where
    E: Expression<'a>,
    C: Cases<'a>,
    T: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("CASE ")?;

        self.expression.build(ctx)?;

        ctx.write_char(' ')?;

        self.cases.build(ctx)?;

        ctx.write_str(" ELSE ")?;

        self.default.build(ctx)?;

        ctx.write_str("END")?;

        Ok(())
    }
}

impl<'a, E, C, T> Ident<'a> for Switch<E, C, T>
where
    E: Expression<'a>,
    C: Cases<'a>,
    T: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Expression<'a>>::build(self, ctx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IfElse<C, T> {
    cases: C,
    default: T,
}

impl<C, T> IfElse<C, T> {
    pub fn new(cases: C, default: T) -> IfElse<C, T> {
        IfElse { cases, default }
    }
}

impl<'a, C, T> Expression<'a> for IfElse<C, T>
where
    C: Cases<'a>,
    T: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("CASE ")?;

        self.cases.build(ctx)?;

        ctx.write_str(" ELSE ")?;

        self.default.build(ctx)?;

        ctx.write_str("END")?;

        Ok(())
    }
}

impl<'a, C, T> Ident<'a> for IfElse<C, T>
where
    C: Cases<'a>,
    T: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        <Self as Expression<'a>>::build(self, ctx)
    }
}

pub trait Cases<'a> {
    fn build(self, context: &mut Context<'a>) -> Result<(), Error>;
}

pub trait Case<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct When<C, T> {
    condition: C,
    value: T,
}

impl<C, T> When<C, T> {
    pub fn new(condition: C, value: T) -> When<C, T> {
        When { condition, value }
    }
}

impl<'a, C, T> Case<'a> for When<C, T>
where
    C: Expression<'a>,
    T: Expression<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("WHEN ")?;
        self.condition.build(ctx)?;
        ctx.write_str(" THEN ")?;
        self.value.build(ctx)?;
        Ok(())
    }
}

macro_rules! cases {
    ($first: ident) => {
        impl<'val, $first: Case<'val>> Cases<'val> for ($first,) {
            #[inline]
            fn build(self, ctx: &mut Context<'val>) -> Result<(),Error> {
                <$first as Case<'val>>::build(self.0, ctx)?;
                Ok(())
            }
        }

    };
    ($first:ident $( $rest: ident )*) => {
        cases!($($rest)*);

        impl<'val,$first: Case<'val>, $( $rest: Case<'val> ),*> Cases<'val> for ($first, $($rest),*) {

            #[allow(non_snake_case)]
            #[inline]
            fn build( self, ctx: &mut Context<'val>) -> Result<(),Error> {
                let ($first, $($rest),*) = self;
                $first.build(ctx)?;
                $(
                    $rest.build(ctx)?;
                )*
                Ok(())
            }
        }
    };
}

cases!(
    C1 C2 C3 C4 C5 C6 C7 C8 C9 C10 C11 C12
);
