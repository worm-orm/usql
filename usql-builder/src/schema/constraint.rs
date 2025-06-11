use core::fmt::Write;

use alloc::{borrow::Cow, vec::Vec};

use crate::{Context, Error};

#[derive(Debug, Clone)]
pub struct Constraint<'a> {
    pub name: Cow<'a, str>,
    pub kind: ConstraintKind<'a>,
}

impl<'a> Constraint<'a> {
    pub fn primary_key(name: impl Into<Cow<'a, str>>, cols: Vec<Cow<'a, str>>) -> Constraint<'a> {
        Constraint {
            name: name.into(),
            kind: ConstraintKind::PrimaryKey(cols),
        }
    }

    pub(crate) fn build(self, ctx: &mut Context<'a>) -> Result<(), Error> {
        ctx.write_str("CONSTRAINT ")?;
        ctx.push_identifier(&self.name)?;
        match self.kind {
            ConstraintKind::PrimaryKey(fields) => {
                write!(ctx, " PRIMARY KEY (")?;
                for (idx, pkg) in fields.into_iter().enumerate() {
                    if idx > 0 {
                        write!(ctx, ",")?;
                    }

                    ctx.push_identifier(&pkg);
                }
                ctx.write_char(')')?;
            }
        };

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ConstraintKind<'a> {
    PrimaryKey(Vec<Cow<'a, str>>),
}
