use core::fmt::Write;

use alloc::{borrow::Cow, vec::Vec};

use crate::{
    Context, Error,
    schema::{column::Column, constraint::Constraint},
    statement::Statement,
};

#[derive(Clone)]
pub struct CreateTable<'a> {
    pub name: Cow<'a, str>,
    pub fields: Vec<Column<'a>>,
    pub force: bool,
    #[allow(unused)]
    pub temporary: bool,
    pub constraints: Vec<Constraint<'a>>,
}

impl<'a> CreateTable<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::default(),
            force: false,
            temporary: false,
            constraints: Vec::default(),
        }
    }
    pub fn column(mut self, field: Column<'a>) -> Self {
        self.fields.push(field);
        self
    }

    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    pub fn constraint(mut self, constraint: Constraint<'a>) -> Self {
        self.constraints.push(constraint);
        self
    }
}

impl<'val> Statement<'val> for CreateTable<'val> {
    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error> {
        ctx.write_str("CREATE")?;
        if self.temporary {
            ctx.write_str(" TEMPORARY")?;
        }

        ctx.write_str(" TABLE ")?;

        if !self.force {
            ctx.write_str("IF NOT EXISTS ")?;
        }

        ctx.push_identifier(&self.name)?;

        ctx.write_str(" (")?;
        let mut fks = Vec::default();
        for (i, v) in self.fields.iter().enumerate() {
            if i > 0 {
                ctx.write_str(", ")?;
            }
            v.build(ctx, false)?;

            if let Some(fk) = &v.foreign_key {
                fks.push((&v.name, fk));
            }
        }
        for (name, fk) in fks.into_iter() {
            write!(ctx, ", FOREIGN KEY (",)?;

            ctx.push_identifier(name)?;

            ctx.write_str(") REFERENCES ")?;

            ctx.push_identifier(&fk.table)?;

            ctx.write_char('(')?;

            ctx.push_identifier(&fk.column)?;

            ctx.write_char(')')?;

            write!(
                ctx,
                " ON DELETE {} ON UPDATE {}",
                fk.on_delete, fk.on_update
            )?;
        }

        for (idx, pkg) in self.constraints.into_iter().enumerate() {
            if idx > 0 {
                write!(ctx, ",")?;
            }
            pkg.build(ctx)?;
        }

        ctx.write_str(")")?;
        Ok(())
    }
}
