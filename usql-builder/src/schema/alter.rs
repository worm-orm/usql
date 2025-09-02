use crate::{Context, Error, expr::Ident, schema::Column, statement::Statement};
use core::fmt::Write;

pub fn alter_table<'a, T, O>(table: T, operation: O) -> AlterTable<T, O> {
    AlterTable { table, operation }
}

pub trait AlterOperation<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error>;
}

pub struct AlterTable<T, O> {
    table: T,
    operation: O,
}

impl<T, O> AlterTable<T, O> {
    pub fn new(table: T, operation: O) -> AlterTable<T, O> {
        AlterTable { table, operation }
    }
}

impl<'a, T, O> Statement<'a> for AlterTable<T, O>
where
    T: Ident<'a>,
    O: AlterOperation<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("ALTER TABLE ")?;

        self.table.build(ctx)?;

        ctx.write_char(' ')?;

        self.operation.build(ctx)?;

        Ok(())
    }
}

pub struct RenameTable<T>(pub T);

impl<'a, T> AlterOperation<'a> for RenameTable<T>
where
    T: Ident<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        write!(ctx, "RENAME TO ")?;
        self.0.build(ctx)?;

        Ok(())
    }
}

impl<'a> AlterOperation<'a> for Column<'a> {
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("ADD COLUMN ")?;

        Self::build(&self, ctx, true)?;

        Ok(())
    }
}

pub struct DropColumn<T>(pub T);

impl<'a, T> AlterOperation<'a> for DropColumn<T>
where
    T: Ident<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("DROP COLUMN ")?;
        self.0.build(ctx)
    }
}

pub struct RenameColumn<T, V>(pub T, pub V);

impl<'a, T, V> AlterOperation<'a> for RenameColumn<T, V>
where
    T: Ident<'a>,
    V: Ident<'a>,
{
    fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("RENAME COLUMN ")?;
        self.0.build(ctx)?;
        ctx.write_str(" TO ")?;
        self.1.build(ctx)
    }
}
