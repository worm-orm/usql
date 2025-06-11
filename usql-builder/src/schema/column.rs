use core::fmt::Write;

use alloc::{borrow::Cow, format};
use usql::{System, ValueCow};

use crate::{
    Context, Error,
    expr::{Expression, ExpressionBox, Ident, expr_box},
    schema::{
        fk::ForeignKey,
        ty::{ColumnType, write_sql_type},
    },
};

#[derive(Clone)]
pub struct Column<'a> {
    pub name: Cow<'a, str>,
    pub kind: ColumnType<'a>,
    pub required: bool,
    pub primary_key: bool,
    pub auto: bool,
    pub default: Option<ExpressionBox<'a>>,
    pub foreign_key: Option<ForeignKey<'a>>,
}

impl<'a> Column<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>, ty: ColumnType<'a>) -> Column<'a> {
        Column {
            name: name.into(),
            kind: ty,
            required: false,
            primary_key: false,
            default: None,
            foreign_key: None,
            auto: false,
        }
    }

    pub fn not_null(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn auto(mut self, auto: bool) -> Self {
        self.auto = auto;
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    pub fn foreign_key(mut self, fk: ForeignKey<'a>) -> Self {
        self.foreign_key = Some(fk);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn default<E>(mut self, value: E) -> Self
    where
        E: Expression<'a> + Send + Sync + Clone + 'a,
    {
        let expr = expr_box(value);
        self.default = Some(expr);
        self
    }
}

impl<'a> Column<'a> {
    pub fn build(&self, ctx: &mut Context<'a>, fk: bool) -> Result<(), Error> {
        ctx.push_identifier(&self.name)?;

        ctx.write_char(' ')?;

        let dialect = ctx.dialect();

        if !(self.auto && dialect == System::Postgres) {
            write_sql_type(&self.kind, ctx, dialect)?;
        } else {
            let kind = match &self.kind {
                ColumnType::SmallInt => "SMALLINT",
                ColumnType::Int => "SERIAL",
                ColumnType::BigInt => "BIGSERIAL",
                ty => return Err(Error::InvalidAutoType(format!("{:?}", ty))),
            };
            ctx.write_str(kind)?;
        }

        if self.primary_key {
            write!(ctx, " PRIMARY KEY")?;
        }

        if !(self.auto && dialect == System::Sqlite) {
            if self.required {
                ctx.write_str(" NOT NULL")?;
            } else if let Some(default) = &self.default {
                ctx.write_str(" DEFAULT ")?;
                default.clone().build(ctx)?;
            } else {
                ctx.write_str(" DEFAULT NULL")?;
            }
        }

        // TODO: Validate type
        if self.auto && dialect == System::Sqlite {
            ctx.write_str(" AUTOINCREMENT")?;
        }

        if let Some(foreign_k) = &self.foreign_key {
            if fk {
                ctx.write_str(" REFERENCES ")?;

                ctx.push_identifier(&foreign_k.table)?;

                ctx.write_char('(')?;

                ctx.push_identifier(&foreign_k.column)?;

                ctx.write_char(')')?;

                write!(
                    ctx,
                    " ON DELETE {} ON UPDATE {}",
                    foreign_k.on_delete, foreign_k.on_update
                )?;
            }
        }

        // if let Some(_default) = &self.default {}

        Ok(())
    }
}
