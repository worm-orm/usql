use std::borrow::Cow;

use usql_builder::{
    expr::raw,
    schema::{self, ColumnType, Constraint, CreateIndex, CreateTable, create_table},
};
use usql_core::System;
use usql_value::Type;

use crate::model::{Auto, Column, ForeignKey, Index, Table, TableKind};

/// A table as statements
pub struct Conversion {
    pub table: CreateTable<'static>,
    pub indexes: Vec<CreateIndex<'static>>,
    /// What is missing from the statements
    pub omitted: Vec<Omitted>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Omitted {
    /// The name of the column, index and so on
    pub name: String,
    pub reason: &'static str,
}

#[derive(Default)]
struct Omit(Vec<Omitted>);

impl Omit {
    fn add(&mut self, name: &str, reason: &'static str) {
        self.0.push(Omitted {
            name: name.to_string(),
            reason,
        });
    }
}

fn is_sqlite(system: System) -> bool {
    matches!(system, System::Sqlite | System::LibSql)
}

impl Table {
    /// Create the table in a database of the given kind.
    ///
    /// Only what can be expressed by the builder is included, the rest is listed. The schema
    /// of the table is never included.
    pub fn to_builder(&self, system: System) -> Conversion {
        let same = self.system == system || (is_sqlite(self.system) && is_sqlite(system));
        let mut omit = Omit::default();

        match self.kind {
            TableKind::Ordinary => {}
            TableKind::Partitioned => omit.add(&self.name, "partitioning"),
            TableKind::Virtual => omit.add(&self.name, "virtual table"),
            TableKind::Foreign => omit.add(&self.name, "foreign table"),
        }
        if self.partition_of.is_some() {
            omit.add(&self.name, "partition");
        }
        if self.strict {
            omit.add(&self.name, "strict");
        }
        if self.without_rowid {
            omit.add(&self.name, "without rowid");
        }
        if self.comment.is_some() {
            omit.add(&self.name, "comment");
        }

        // A key with another name than the default must be a constraint
        let key = self.primary_key.as_ref();
        let key_name = key
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| format!("{}_pkey", self.name));
        let inline = key
            .filter(|key| key.columns.len() == 1)
            .filter(|_| key_name == format!("{}_pkey", self.name))
            .map(|key| key.columns[0].as_str());

        let mut table = create_table(self.name.clone());
        for column in &self.columns {
            if column.generated.is_some() {
                omit.add(&column.name, "generated column");
                continue;
            }
            let primary = inline == Some(column.name.as_str());
            let mut output = convert_column(column, system, same, primary, &mut omit);
            for fk in &self.foreign_keys {
                if fk.columns != [column.name.as_str()] {
                    continue;
                }
                if output.foreign_key.is_some() {
                    omit.add(&fk_name(fk), "more than one foreign key on a column");
                } else if fk.ref_schema != self.schema {
                    omit.add(&fk_name(fk), "foreign key to another schema");
                } else if let [ref_column] = fk.ref_columns.as_slice() {
                    if fk
                        .name
                        .as_ref()
                        .is_some_and(|name| !self.is_default_fk_name(name, &column.name))
                    {
                        omit.add(&fk_name(fk), "name of foreign key");
                    }
                    output = output.foreign_key(
                        schema::ForeignKey::new(fk.ref_table.clone(), ref_column.clone())
                            .on_delete(fk.on_delete)
                            .on_update(fk.on_update),
                    );
                } else {
                    omit.add(&fk_name(fk), "foreign key without columns");
                }
            }
            table = table.column(output);
        }

        if let Some(key) = key
            && inline.is_none()
        {
            let columns = key.columns.iter().cloned().map(Cow::Owned).collect();
            table = table.constraint(Constraint::primary_key(key_name, columns));
        }
        for fk in &self.foreign_keys {
            if fk.columns.len() != 1 {
                omit.add(&fk_name(fk), "composite foreign key");
            }
        }
        for key in &self.uniques {
            let name = key.name.clone().unwrap_or_else(|| key.columns.join(","));
            omit.add(&name, "unique constraint");
        }
        for trigger in &self.triggers {
            omit.add(&trigger.name, "trigger");
        }

        let mut indexes = Vec::new();
        for index in &self.indexes {
            match self.convert_index(index) {
                Some(index) => indexes.push(index),
                None => omit.add(&index.name, "index"),
            }
        }

        Conversion {
            table,
            indexes,
            omitted: omit.0,
        }
    }

    /// As given by the database, when the foreign key has no name
    fn is_default_fk_name(&self, name: &str, column: &str) -> bool {
        match self.system {
            // As `post_ibfk_1`
            System::Mysql => name
                .strip_prefix(self.name.as_str())
                .and_then(|m| m.strip_prefix("_ibfk_"))
                .is_some_and(|m| !m.is_empty() && m.bytes().all(|b| b.is_ascii_digit())),
            _ => name == format!("{}_{}_fkey", self.name, column),
        }
    }

    fn convert_index(&self, index: &Index) -> Option<CreateIndex<'static>> {
        if index.method != "btree" || index.partial || !index.included.is_empty() {
            return None;
        }
        let columns = index
            .parts
            .iter()
            .map(|part| {
                let plain = !part.descending && part.collation.is_none();
                part.column.as_ref().filter(|_| plain)
            })
            .collect::<Option<Vec<_>>>()?;

        if columns.iter().any(|m| {
            !self
                .columns
                .iter()
                .any(|c| c.name == **m && c.generated.is_none())
        }) {
            return None;
        }

        // Any other option, as operator classes, is also part of the definition
        if self.system == System::Postgres {
            let definition = index.definition.as_deref()?;
            let quote = |name: &str| format!("\"{}\"", name.replace('"', "\"\""));
            let mut rest = definition.rsplit_once(" USING btree (")?.1;
            for (idx, name) in columns.iter().enumerate() {
                if idx > 0 {
                    rest = rest.strip_prefix(", ")?;
                }
                rest = rest
                    .strip_prefix(quote(name).as_str())
                    .or_else(|| rest.strip_prefix(name.as_str()))?;
            }
            if rest != ")" {
                return None;
            }
        }

        let columns = columns.into_iter().cloned().map(Cow::Owned).collect();
        Some(CreateIndex::new(self.name.clone(), index.name.clone(), columns).unique(index.unique))
    }
}

fn fk_name(fk: &ForeignKey) -> String {
    fk.name.clone().unwrap_or_else(|| fk.columns.join(","))
}

fn convert_column(
    column: &Column,
    system: System,
    same: bool,
    primary: bool,
    omit: &mut Omit,
) -> schema::Column<'static> {
    let mapped = column.ty.as_ref().and_then(column_type);

    let mut auto = column.auto.is_some();
    if column.auto == Some(Auto::Identity) {
        omit.add(&column.name, "identity");
    }
    // MySQL also needs a key, and the builder only has primary keys
    if auto && (is_sqlite(system) || system == System::Mysql) && !primary {
        omit.add(&column.name, "auto increment");
        auto = false;
    }
    let is_int = matches!(
        mapped,
        Some(ColumnType::SmallInt | ColumnType::Int | ColumnType::BigInt)
    );
    if auto && !is_int {
        omit.add(&column.name, "auto increment");
        auto = false;
    }

    // The builder only makes these from the integer types
    if auto && same && system == System::Mysql {
        // MariaDB adds the display width, as `bigint(20)`
        let name = column.type_name.as_str();
        let name = name
            .strip_suffix(')')
            .and_then(|m| m.rsplit_once('('))
            .filter(|(_, width)| !width.is_empty() && width.bytes().all(|b| b.is_ascii_digit()))
            .map_or(name, |(name, _)| name);
        let plain = ["smallint", "int", "bigint"].contains(&name);
        if !plain {
            omit.add(&column.name, "type of auto increment column");
        }
    }

    let kind = if auto && matches!(system, System::Postgres | System::Mysql) {
        mapped
    } else if same {
        Some(ColumnType::Other(column.type_name.clone().into()))
    } else {
        mapped
    };
    let kind = kind.unwrap_or_else(|| {
        omit.add(&column.name, "type, which is replaced by text");
        ColumnType::Text
    });

    let mut output = schema::Column::new(column.name.clone(), kind)
        .auto(auto)
        .required(!column.nullable);
    if primary {
        output = output.primary_key();
    }
    if let Some(default) = &column.default {
        if same || is_literal(default) {
            output = output.default(raw(default.clone()));
        } else {
            omit.add(&column.name, "default");
        }
    }
    if column.on_update.is_some() {
        omit.add(&column.name, "on update");
    }
    if column.comment.is_some() {
        omit.add(&column.name, "comment");
    }
    output
}

fn column_type(ty: &Type) -> Option<ColumnType<'static>> {
    Some(match ty {
        Type::Text => ColumnType::Text,
        Type::SmallInt => ColumnType::SmallInt,
        Type::Int => ColumnType::Int,
        Type::BigInt => ColumnType::BigInt,
        Type::Blob => ColumnType::Blob,
        Type::Time => ColumnType::Time,
        Type::Date => ColumnType::Date,
        Type::DateTime => ColumnType::DateTime,
        Type::Json => ColumnType::Json,
        Type::Float => ColumnType::Float,
        Type::Double => ColumnType::Double,
        Type::Uuid => ColumnType::Uuid,
        Type::Bool => ColumnType::Bool,
        Type::Array(_) | Type::Geometry(_) | Type::Any => return None,
    })
}

/// Means the same everywhere. MySQL reads a backslash in a string as an escape
fn is_literal(sql: &str) -> bool {
    let string = sql
        .strip_prefix('\'')
        .and_then(|m| m.strip_suffix('\''))
        .is_some_and(|m| !m.replace("''", "").contains('\'') && !m.contains('\\'));
    let digits = sql.strip_prefix('-').unwrap_or(sql);
    let number = digits.starts_with(|m: char| m.is_ascii_digit())
        && digits.bytes().all(|m| m.is_ascii_digit() || m == b'.')
        && digits.parse::<f64>().is_ok();
    string || number
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Key;
    use usql_builder::StatementExt;

    fn column(name: &str, type_name: &str, ty: Type, auto: bool) -> Column {
        Column {
            name: name.to_string(),
            position: 1,
            type_name: type_name.to_string(),
            ty: Some(ty),
            nullable: !auto,
            default: None,
            auto: auto.then_some(Auto::AutoIncrement),
            generated: None,
            on_update: None,
            comment: None,
        }
    }

    fn table(columns: Vec<Column>, foreign_keys: Vec<ForeignKey>) -> Table {
        Table {
            system: System::Mysql,
            schema: "db".to_string(),
            name: "t".to_string(),
            kind: TableKind::Ordinary,
            comment: None,
            partition_of: None,
            strict: false,
            without_rowid: false,
            primary_key: Some(Key {
                name: None,
                columns: vec![columns[0].name.clone()],
            }),
            columns,
            foreign_keys,
            uniques: Vec::new(),
            indexes: Vec::new(),
            triggers: Vec::new(),
        }
    }

    fn omitted(table: &Table, system: System) -> Vec<(String, &'static str)> {
        table
            .to_builder(system)
            .omitted
            .into_iter()
            .map(|m| (m.name, m.reason))
            .collect()
    }

    #[test]
    fn mysql_auto_increment() {
        for (type_name, ty, plain) in [
            ("bigint", Type::BigInt, true),
            ("bigint(20)", Type::BigInt, true),
            ("int unsigned", Type::BigInt, false),
            ("int(10) unsigned", Type::BigInt, false),
            ("tinyint", Type::SmallInt, false),
        ] {
            let t = table(vec![column("id", type_name, ty, true)], Vec::new());
            let expected = match plain {
                true => vec![],
                false => vec![("id".to_string(), "type of auto increment column")],
            };
            assert_eq!(omitted(&t, System::Mysql), expected, "{type_name}");
            let sql = t
                .to_builder(System::Mysql)
                .table
                .to_sql(System::Mysql)
                .unwrap();
            assert!(sql.sql.contains("AUTO_INCREMENT"), "{}", sql.sql);
        }

        // Only a primary key can be auto increment
        let mut t = table(
            vec![
                column("a", "int", Type::Int, false),
                column("b", "int", Type::Int, true),
            ],
            Vec::new(),
        );
        t.columns[0].nullable = false;
        assert_eq!(
            omitted(&t, System::Mysql),
            [("b".to_string(), "auto increment")]
        );
    }

    #[test]
    fn mysql_foreign_key_names() {
        for (name, omit) in [
            ("t_ibfk_1", false),
            ("t_ibfk_12", false),
            ("t_ibfk_", true),
            ("fk", true),
        ] {
            let fk = ForeignKey {
                name: Some(name.to_string()),
                columns: vec!["p".to_string()],
                ref_schema: "db".to_string(),
                ref_table: "p".to_string(),
                ref_columns: vec!["id".to_string()],
                on_update: usql_builder::schema::ReferentialAction::NoAction,
                on_delete: usql_builder::schema::ReferentialAction::NoAction,
            };
            let t = table(
                vec![
                    column("id", "int", Type::Int, false),
                    column("p", "int", Type::Int, false),
                ],
                vec![fk],
            );
            let expected = match omit {
                true => vec![(name.to_string(), "name of foreign key")],
                false => vec![],
            };
            assert_eq!(omitted(&t, System::Mysql), expected, "{name}");
        }
    }

    #[test]
    fn literals() {
        assert!(is_literal("'it''s'") && is_literal("-1.5"));
        // A backslash is an escape on MySQL only
        assert!(!is_literal("'a\\b'"));
    }
}
