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
                    let default = format!("{}_{}_fkey", self.name, column.name);
                    if fk.name.as_ref().is_some_and(|name| *name != default) {
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
    if auto && is_sqlite(system) && !primary {
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

    let kind = if auto && system == System::Postgres {
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

/// Means the same everywhere
fn is_literal(sql: &str) -> bool {
    let string = sql
        .strip_prefix('\'')
        .and_then(|m| m.strip_suffix('\''))
        .is_some_and(|m| !m.replace("''", "").contains('\''));
    let digits = sql.strip_prefix('-').unwrap_or(sql);
    let number = digits.starts_with(|m: char| m.is_ascii_digit())
        && digits.bytes().all(|m| m.is_ascii_digit() || m == b'.')
        && digits.parse::<f64>().is_ok();
    string || number
}
