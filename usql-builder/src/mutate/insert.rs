use core::fmt::Write;

use alloc::{borrow::Cow, vec::Vec};

use crate::{
    Context, Error,
    expr::{ExpressionBox, expr_box},
    mutate::set::Set,
    select::Selection,
    statement::Statement,
};

pub fn insert<'key, 'val>(table: impl Into<Cow<'key, str>>) -> Insert<'key, 'val> {
    Insert::new(table)
}

#[derive(Clone)]
pub struct Insert<'key, 'val> {
    pub(crate) table: Cow<'key, str>,
    pub(crate) keys: Vec<Cow<'key, str>>,
    pub(crate) values: Vec<ExpressionBox<'val>>,
}

impl<'key, 'val> Set<'key, 'val> for Insert<'key, 'val> {
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'key, str>>,
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

impl<'key, 'val> Insert<'key, 'val> {
    pub fn new(table: impl Into<Cow<'key, str>>) -> Insert<'key, 'val> {
        Insert {
            table: table.into(),
            values: Vec::default(),
            keys: Vec::default(),
        }
    }

    pub fn returning<T>(self, selection: T) -> InsertReturning<'key, 'val, T>
    where
        T: Selection<'val>,
    {
        InsertReturning {
            insert: self,
            returning: selection,
        }
    }
}

impl<'key, 'val> Statement<'val> for Insert<'key, 'val> {
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
pub struct InsertReturning<'key, 'val, S> {
    insert: Insert<'key, 'val>,
    returning: S,
}

impl<'key, 'val, S> Set<'key, 'val> for InsertReturning<'key, 'val, S>
where
    S: Selection<'val>,
{
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'key, str>>,
        V: crate::expr::Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.insert.set(field, value);
        self
    }

    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <Self as Statement>::build(self, ctx)
    }
}

impl<'key, 'val, S> Statement<'val> for InsertReturning<'key, 'val, S>
where
    S: Selection<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <Insert<'key, 'val> as Statement>::build(self.insert, ctx)?;
        write!(ctx, " RETURNING ")?;
        self.returning.build(ctx)?;
        Ok(())
    }
}
