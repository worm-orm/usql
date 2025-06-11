use core::fmt::Write;

use alloc::{borrow::Cow, vec::Vec};

use crate::{
    Context, Error,
    expr::{ExpressionBox, expr_box},
    mutate::set::Set,
    select::Selection,
    statement::Statement,
};

pub fn insert<'val>(table: impl Into<Cow<'val, str>>) -> Insert<'val> {
    Insert::new(table)
}

#[derive(Clone)]
pub struct Insert<'val> {
    pub(crate) table: Cow<'val, str>,
    pub(crate) keys: Vec<Cow<'val, str>>,
    pub(crate) values: Vec<ExpressionBox<'val>>,
}

impl<'val> Set<'val> for Insert<'val> {
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'val, str>>,
        V: crate::expr::Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.keys.push(field.into());
        self.values.push(expr_box(value));
        self
    }

    fn build(self, ctx: &mut crate::Context<'val>) -> Result<(), crate::Error> {
        <Self as Statement>::build(self, ctx)
    }
}

impl<'val> Insert<'val> {
    pub fn new(table: impl Into<Cow<'val, str>>) -> Insert<'val> {
        Insert {
            table: table.into(),
            values: Vec::default(),
            keys: Vec::default(),
        }
    }

    pub fn returning<T>(self, selection: T) -> InsertReturning<'val, T>
    where
        T: Selection<'val>,
    {
        InsertReturning {
            insert: self,
            returning: selection,
        }
    }
}

impl<'val> Statement<'val> for Insert<'val> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        write!(ctx, "INSERT INTO ")?;
        self.table.build(ctx)?;
        write!(ctx, " (")?;
        for (idx, value) in self.keys.iter().enumerate() {
            if idx > 0 {
                ctx.write_str(",")?;
            }
            value.build(ctx)?;
        }

        write!(ctx, ") VALUES (")?;

        for (idx, value) in self.values.into_iter().enumerate() {
            if idx > 0 {
                ctx.write_str(",")?;
            }

            value.build(ctx)?;
        }
        ctx.write_str(")")?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct InsertReturning<'val, S> {
    insert: Insert<'val>,
    returning: S,
}

impl<'val, S> Set<'val> for InsertReturning<'val, S>
where
    S: Selection<'val>,
{
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'val, str>>,
        V: crate::expr::Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.insert.set(field, value);
        self
    }

    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <Self as Statement>::build(self, ctx)
    }
}

impl<'val, S> Statement<'val> for InsertReturning<'val, S>
where
    S: Selection<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <Insert<'val> as Statement>::build(self.insert, ctx)?;
        write!(ctx, " RETURNING ")?;
        self.returning.build(ctx)?;
        Ok(())
    }
}
