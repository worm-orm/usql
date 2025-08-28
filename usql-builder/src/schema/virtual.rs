use core::fmt::Write;

use alloc::{borrow::Cow, vec::Vec};

use crate::statement::Statement;

pub fn create_virtual_table<'a>(
    name: impl Into<Cow<'a, str>>,
    module: impl Into<Cow<'a, str>>,
) -> CreateVirtualTable<'a> {
    CreateVirtualTable::new(name, module)
}

pub struct CreateVirtualTable<'a> {
    name: Cow<'a, str>,
    force: bool,
    module: Cow<'a, str>,
    args: Vec<Cow<'a, str>>,
}

impl<'a> CreateVirtualTable<'a> {
    pub fn new(
        name: impl Into<Cow<'a, str>>,
        module: impl Into<Cow<'a, str>>,
    ) -> CreateVirtualTable<'a> {
        CreateVirtualTable {
            name: name.into(),
            force: false,
            module: module.into(),
            args: Default::default(),
        }
    }

    pub fn arg(mut self, arg: impl Into<Cow<'a, str>>) -> Self {
        self.args.push(arg.into());
        self
    }
}

impl<'a> Statement<'a> for CreateVirtualTable<'a> {
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        ctx.write_str("CREATE VIRTUAL TABLE ")?;
        if !self.force {
            ctx.write_str("IF NOT EXISTS ")?;
        }

        ctx.push_identifier(&self.name)?;

        write!(ctx, " USING {}", self.module)?;

        if !self.args.is_empty() {
            ctx.write_char('(')?;
            for (idx, next) in self.args.into_iter().enumerate() {
                if idx > 0 {
                    ctx.write_char(',')?;
                }

                ctx.write_str(&next)?;
            }
            ctx.write_char(')')?;
        }

        Ok(())
    }
}
