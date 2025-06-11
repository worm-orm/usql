use alloc::{borrow::Cow, vec::Vec};

use crate::{Context, Error, statement::Statement};

use core::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub struct CreateIndex<'a> {
    name: Cow<'a, str>,
    unique: bool,
    table: Cow<'a, str>,
    columns: Vec<Cow<'a, str>>,
}

impl<'a> CreateIndex<'a> {
    pub fn new(
        table: impl Into<Cow<'a, str>>,
        name: impl Into<Cow<'a, str>>,
        columns: Vec<Cow<'a, str>>,
    ) -> CreateIndex<'a> {
        CreateIndex {
            name: name.into(),
            table: table.into(),
            unique: false,
            columns,
        }
    }

    pub fn unique(mut self, unique: bool) -> Self {
        self.unique = unique;
        self
    }
}

impl<'a> Statement<'a> for CreateIndex<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("CREATE ")?;
        if self.unique {
            ctx.write_str("UNIQUE ")?;
        }

        ctx.write_str("INDEX IF NOT EXISTS ")?;

        ctx.push_identifier(&self.name)?;

        ctx.write_str(" ON ")?;

        ctx.push_identifier(&self.table)?;

        ctx.write_char('(')?;

        for (k, v) in self.columns.iter().enumerate() {
            if k != 0 {
                ctx.write_char(',')?;
            }

            ctx.push_identifier(&v)?;
        }

        ctx.write_char(')')?;

        Ok(())
    }
}

pub struct DropIndex<'a> {
    index: Cow<'a, str>,
}

impl<'a> DropIndex<'a> {
    pub fn new(index: impl Into<Cow<'a, str>>) -> DropIndex<'a> {
        DropIndex {
            index: index.into(),
        }
    }
}

impl<'a> Statement<'a> for DropIndex<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("DROP INDEX ")?;

        ctx.push_identifier(&self.index)?;

        Ok(())
    }
}
