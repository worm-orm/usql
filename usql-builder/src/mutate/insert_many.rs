use core::fmt::Write;

use alloc::vec::Vec;
use usql_value::ValueCow;

use crate::{
    Context, Error,
    expr::{Expression, Ident},
    statement::Statement,
};

pub trait Fields<'val> {
    fn len(&self) -> usize;
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error>;
}

impl<'val, T> Fields<'val> for Vec<T>
where
    T: Ident<'val>,
{
    fn len(&self) -> usize {
        self.len()
    }

    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        for (idx, value) in self.into_iter().enumerate() {
            if idx > 0 {
                ctx.write_str(",")?;
            }
            value.build(ctx)?;
        }

        Ok(())
    }
}

impl<'val, T, const LEN: usize> Fields<'val> for [T; LEN]
where
    T: Ident<'val>,
{
    fn len(&self) -> usize {
        LEN
    }

    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        for (idx, value) in self.into_iter().enumerate() {
            if idx > 0 {
                ctx.write_str(",")?;
            }
            value.build(ctx)?;
        }

        Ok(())
    }
}

pub trait ValueList<'val> {
    fn len(&self) -> usize;
    fn into_list(self) -> Vec<ValueCow<'val>>;
}

impl<'val, T> ValueList<'val> for Vec<T>
where
    T: Into<ValueCow<'val>>,
{
    fn len(&self) -> usize {
        self.len()
    }

    fn into_list(self) -> Vec<ValueCow<'val>> {
        self.into_iter().map(|m| m.into()).collect()
    }
}

impl<'val, T, const LEN: usize> ValueList<'val> for [T; LEN]
where
    T: Into<ValueCow<'val>>,
{
    fn len(&self) -> usize {
        LEN
    }

    fn into_list(self) -> Vec<ValueCow<'val>> {
        self.into_iter().map(|m| m.into()).collect()
    }
}

#[derive(Debug)]
pub struct InsertMany<'val, T, K: Fields<'val>> {
    table: T,
    keys: K,
    values: Vec<Vec<ValueCow<'val>>>,
}

impl<'val, T, K: Fields<'val>> InsertMany<'val, T, K> {
    pub fn new(table: T, keys: K) -> InsertMany<'val, T, K> {
        InsertMany {
            table,
            keys,
            values: Vec::new(),
        }
    }

    pub fn values(mut self, values: impl ValueList<'val>) -> Result<Self, Error> {
        if values.len() != self.keys.len() {
            return Err(Error::InvalidValueCount {
                expected: self.keys.len(),
                found: values.len(),
            });
        };

        self.values.push(values.into_list());

        Ok(self)
    }
}

impl<'val, T: Ident<'val>, K: Fields<'val>> Statement<'val> for InsertMany<'val, T, K> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        write!(ctx, "INSERT INTO ")?;
        self.table.build(ctx)?;
        write!(ctx, " (")?;

        self.keys.build(ctx)?;

        write!(ctx, ") VALUES ")?;

        for (idx, value) in self.values.into_iter().enumerate() {
            if idx > 0 {
                ctx.write_str(",")?;
            }

            ctx.write_char('(')?;

            for (idx, value) in value.into_iter().enumerate() {
                if idx > 0 {
                    ctx.write_str(",")?;
                }

                <ValueCow<'val> as Expression<'val>>::build(value, ctx)?;
            }

            ctx.write_char(')')?;
        }

        Ok(())
    }
}
