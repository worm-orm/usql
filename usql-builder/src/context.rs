use alloc::{
    fmt::{self, Write},
    string::String,
    vec::Vec,
};

use usql_core::System;
use usql_value::ValueCow;

use crate::{error::Error, sql::SqlStmt};

pub struct Context<'a> {
    system: System,
    bindings: Vec<ValueCow<'a>>,
    output: String,
}

impl<'a> Context<'a> {
    pub fn new(system: System) -> Context<'a> {
        Context {
            system,
            bindings: Default::default(),
            output: Default::default(),
        }
    }

    pub fn dialect(&self) -> System {
        self.system
    }
    pub fn push<S: Into<ValueCow<'a>>>(&mut self, value: S) -> Result<(), Error> {
        let value = value.into();

        if !value.as_ref().is_null() {
            self.bindings.push(value);
            match self.system {
                System::Mysql | System::LibSql | System::Sqlite => self.write_str("?"),
                System::Postgres => {
                    write!(self.output, "${}", self.bindings.len())
                }
            }?;
        } else {
            self.write_str("NULL")?;
        }

        Ok(())
    }

    pub fn push_identifier(&mut self, identifier: &str) -> Result<(), Error>
    where
        Self: Sized,
    {
        write_identifier(identifier, &self.system, &mut self.output)?;
        Ok(())
    }

    pub fn build(self) -> SqlStmt<'a> {
        SqlStmt::new(self.output, self.bindings)
    }
}

impl<'a> fmt::Display for Context<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.output.fmt(f)
    }
}

impl<'a> Write for Context<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.output.write_str(s)
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.output.write_char(c)
    }

    fn write_fmt(&mut self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        self.output.write_fmt(args)
    }
}

pub fn write_identifier<W>(ident: &str, dialect: &System, out: &mut W) -> fmt::Result
where
    W: Write + ?Sized,
{
    let quote = match dialect {
        System::Mysql => '`',
        System::Sqlite | System::LibSql | System::Postgres => '"',
    };
    out.write_char(quote)?;
    for (idx, part) in ident.split(quote).enumerate() {
        if idx > 0 {
            // Escaped by doubling it
            out.write_char(quote)?;
            out.write_char(quote)?;
        }
        out.write_str(part)?;
    }
    out.write_char(quote)
}

// pub fn write_identifier_escape<W>(dialect: &System, out: &mut W) -> fmt::Result
// where
//     W: Write + ?Sized,
// {
//     match dialect {
//         &System::Mysql => {
//             write!(out, "`")
//         }
//         &System::Sqlite | &System::LibSql | &System::Postgres => {
//             write!(out, "\"")
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        StatementExt,
        expr::raw,
        mutate::{Set, insert},
        schema::{
            Column, ColumnType, Constraint, CreateIndex, DropIndex, ForeignKey, ReferentialAction,
            alter_table, create_table,
        },
    };
    use usql_value::Value;

    #[test]
    fn placeholders() {
        let build = |system| {
            insert("test")
                .with("a", Value::Int(1))
                .with("b", Value::Null)
                .with("c", Value::Int(2))
                .to_sql(system)
                .unwrap()
        };
        let sql = build(System::Postgres);
        assert_eq!(
            sql.sql,
            r#"INSERT INTO "test" ("a","b","c") VALUES ($1,NULL,$2)"#
        );
        assert_eq!(sql.bindings.len(), 2);
        assert_eq!(
            build(System::Sqlite).sql,
            r#"INSERT INTO "test" ("a","b","c") VALUES (?,NULL,?)"#
        );
    }

    #[test]
    fn auto_columns() {
        let build = |kind, system| {
            create_table("test")
                .column(Column::new("id", kind).primary_key().auto(true))
                .to_sql(system)
                .unwrap()
                .sql
        };
        assert_eq!(
            build(ColumnType::SmallInt, System::Postgres),
            r#"CREATE TABLE IF NOT EXISTS "test" ("id" SMALLSERIAL PRIMARY KEY)"#
        );
        assert_eq!(
            build(ColumnType::Int, System::Postgres),
            r#"CREATE TABLE IF NOT EXISTS "test" ("id" SERIAL PRIMARY KEY)"#
        );
        assert_eq!(
            build(ColumnType::Int, System::Sqlite),
            r#"CREATE TABLE IF NOT EXISTS "test" ("id" INTEGER PRIMARY KEY AUTOINCREMENT)"#
        );
    }

    #[test]
    fn identifiers() {
        for (system, name, expected) in [
            (System::Postgres, "test", "\"test\""),
            (System::Sqlite, "a\"b`", "\"a\"\"b`\""),
            (System::Mysql, "a\"b`", "`a\"b```"),
        ] {
            let mut ctx = Context::new(system);
            ctx.push_identifier(name).unwrap();
            assert_eq!(ctx.build().sql, expected);
        }
    }

    #[test]
    fn defaults_and_constraints() {
        let sql = create_table("test")
            .column(
                Column::new("a", ColumnType::Int)
                    .required(true)
                    .default(raw("1 + 1")),
            )
            .column(Column::new("b", ColumnType::Int).required(true))
            .column(Column::new("c", ColumnType::Int))
            .constraint(Constraint::primary_key(
                "pk",
                alloc::vec!["a".into(), "b".into()],
            ))
            .to_sql(System::Postgres)
            .unwrap()
            .sql;
        assert_eq!(
            sql,
            r#"CREATE TABLE IF NOT EXISTS "test" ("a" INTEGER NOT NULL DEFAULT (1 + 1), "b" INTEGER NOT NULL, "c" INTEGER DEFAULT NULL, CONSTRAINT "pk" PRIMARY KEY ("a","b"))"#
        );
    }

    #[test]
    fn mysql_create_table() {
        let sql = create_table("post")
            .column(
                Column::new("id", ColumnType::BigInt)
                    .primary_key()
                    .auto(true),
            )
            .column(Column::new("title", ColumnType::VarChar(200)).required(true))
            .column(Column::new("body", ColumnType::Text))
            .column(
                Column::new("published", ColumnType::Bool)
                    .required(true)
                    .default(raw("FALSE")),
            )
            .column(Column::new("created", ColumnType::DateTime).required(true))
            .column(Column::new("key", ColumnType::Uuid).required(true))
            .column(Column::new("meta", ColumnType::Json))
            .column(
                Column::new("user_id", ColumnType::Int)
                    .required(true)
                    .foreign_key(
                        ForeignKey::new("user", "id").on_delete(ReferentialAction::Cascade),
                    ),
            )
            .to_sql(System::Mysql)
            .unwrap();
        assert_eq!(
            sql.sql,
            "CREATE TABLE IF NOT EXISTS `post` (`id` BIGINT PRIMARY KEY AUTO_INCREMENT, \
             `title` VARCHAR(200) NOT NULL, `body` LONGTEXT DEFAULT NULL, \
             `published` BOOLEAN NOT NULL DEFAULT (FALSE), `created` DATETIME(6) NOT NULL, \
             `key` BINARY(16) NOT NULL, `meta` JSON DEFAULT NULL, `user_id` INT NOT NULL, \
             FOREIGN KEY (`user_id`) REFERENCES `user`(`id`) ON DELETE CASCADE ON UPDATE NO ACTION)"
        );
        assert!(sql.bindings.is_empty());
    }

    #[test]
    fn mysql_auto_requires_integer() {
        let result = create_table("test")
            .column(Column::new("id", ColumnType::Text).primary_key().auto(true))
            .to_sql(System::Mysql);
        assert!(matches!(result, Err(Error::InvalidAutoType(_))));
    }

    #[test]
    fn mysql_add_column_with_foreign_key() {
        let sql = alter_table(
            "post",
            Column::new("user_id", ColumnType::Int).foreign_key(ForeignKey::new("user", "id")),
        )
        .to_sql(System::Mysql)
        .unwrap()
        .sql;
        assert_eq!(
            sql,
            "ALTER TABLE `post` ADD COLUMN `user_id` INT DEFAULT NULL, ADD FOREIGN KEY (`user_id`) \
             REFERENCES `user`(`id`) ON DELETE NO ACTION ON UPDATE NO ACTION"
        );
    }

    #[test]
    fn mysql_indexes() {
        let create = |system| {
            CreateIndex::new("post", "post_title", alloc::vec!["title".into()])
                .unique(true)
                .to_sql(system)
                .unwrap()
                .sql
        };
        assert_eq!(
            create(System::Mysql),
            "CREATE UNIQUE INDEX `post_title` ON `post`(`title`)"
        );
        assert_eq!(
            create(System::Postgres),
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "post_title" ON "post"("title")"#
        );
        assert_eq!(
            DropIndex::new("post_title")
                .table("post")
                .to_sql(System::Mysql)
                .unwrap()
                .sql,
            "DROP INDEX `post_title` ON `post`"
        );
        assert_eq!(
            DropIndex::new("post_title")
                .table("post")
                .to_sql(System::Postgres)
                .unwrap()
                .sql,
            r#"DROP INDEX "post_title""#
        );
        assert!(DropIndex::new("post_title").to_sql(System::Mysql).is_err());
    }

    #[test]
    fn mysql_placeholders() {
        let sql = insert("test")
            .with("a", Value::Int(1))
            .with("b", Value::Null)
            .to_sql(System::Mysql)
            .unwrap();
        assert_eq!(sql.sql, "INSERT INTO `test` (`a`,`b`) VALUES (?,NULL)");
        assert_eq!(sql.bindings.len(), 1);
    }
}
