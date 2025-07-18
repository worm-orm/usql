use alloc::{borrow::Cow, fmt::Write, vec::Vec};

use crate::{
    Context, Error,
    expr::{Expression, ExpressionBox, expr_box},
    mutate::Set,
    select::Selection,
    statement::Statement,
};

#[derive(Clone)]
pub struct Update<'val> {
    pub(crate) table: Cow<'val, str>,
    pub(crate) keys: Vec<Cow<'val, str>>,
    pub(crate) values: Vec<ExpressionBox<'val>>,
}

impl<'val> Set<'val> for Update<'val> {
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'val, str>>,
        V: crate::expr::Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.keys.push(field.into());
        self.values.push(expr_box(value));
        self
    }
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <Self as Statement>::build(self, ctx)
    }
}

impl<'val> Update<'val> {
    pub fn new(table: impl Into<Cow<'val, str>>) -> Update<'val> {
        Update {
            table: table.into(),
            values: Vec::default(),
            keys: Vec::default(),
        }
    }

    pub fn returning<S>(self, selection: S) -> UpdateReturning<S, Self>
    where
        S: Selection<'val>,
    {
        UpdateReturning {
            update: self,
            returning: selection,
        }
    }

    pub fn filter<'a, E: Expression<'a>>(self, expr: E) -> UpdateFilter<'val, E> {
        UpdateFilter {
            update: self,
            filter: expr,
        }
    }
}

impl<'val> Statement<'val> for Update<'val> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        write!(ctx, "UPDATE ")?;
        ctx.push_identifier(&self.table)?;
        write!(ctx, " SET ")?;
        for (idx, value) in self.keys.iter().enumerate() {
            if idx > 0 {
                ctx.write_str(",")?;
            }
            ctx.push_identifier(value)?;
            write!(ctx, " = ")?;
            self.values[idx].clone().build(ctx)?;
        }

        Ok(())
    }
}

pub struct UpdateFilter<'val, E> {
    update: Update<'val>,
    filter: E,
}

impl<'val, E> UpdateFilter<'val, E> {
    pub fn returning<S>(self, selection: S) -> UpdateReturning<S, Self>
    where
        S: Selection<'val>,
    {
        UpdateReturning {
            update: self,
            returning: selection,
        }
    }
}

impl<'val, E> Set<'val> for UpdateFilter<'val, E>
where
    E: Expression<'val>,
{
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'val, str>>,
        V: Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.update.set(field, value);
        self
    }

    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <Self as Statement>::build(self, ctx)
    }
}

impl<'val, E> Statement<'val> for UpdateFilter<'val, E>
where
    E: Expression<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        <Update as Statement>::build(self.update, ctx)?;
        write!(ctx, " WHERE ")?;
        self.filter.build(ctx)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UpdateReturning<S, U> {
    update: U,
    returning: S,
}

impl<'val, S, U> UpdateReturning<S, U>
where
    U: Set<'val>,
{
    pub fn with<F, V>(mut self, field: F, value: V) -> Self
    where
        F: Into<Cow<'val, str>>,
        V: Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.update = self.update.with(field, value);
        self
    }

    pub fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'val, str>>,
        V: Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.update.set(field, value);
        self
    }

    // pub fn to_static(self) -> UpdateReturning<'static, S, U> {
    //     UpdateReturning {
    //         update: self.update.to_static(),
    //         returning: self.returning,
    //     }
    // }
}

impl<'val, S, U> Statement<'val> for UpdateReturning<S, U>
where
    S: Selection<'val>,
    U: Statement<'val>,
{
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        self.update.build(ctx)?;
        write!(ctx, " RETURNING ")?;
        self.returning.build(ctx)?;
        Ok(())
    }
}

pub fn update<'val>(table: impl Into<Cow<'val, str>>) -> Update<'val> {
    Update::new(table)
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     // use crate::build::*;
//     use worm_database::Dialect;

//     #[test]
//     fn test() {
//         let mut output = crate::build(Dialect::Sqlite, Update::new("blogs").set("name", "Rasmus"));

//         println!("oUTPUT {:?}", output);
//     }
// }
