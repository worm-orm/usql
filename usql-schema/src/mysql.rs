//! MySQL and MariaDB, from `information_schema`.
//!
//! A schema is a database. MySQL makes no difference between a unique constraint and a
//! unique index, so both are read as unique indexes. The indexes MySQL adds for foreign keys
//! are left out, and enums are part of the column type.

use std::collections::HashMap;

use usql_builder::schema::ReferentialAction;
use usql_core::{DatabaseInfo, Executor};
use usql_value::{Type, Value, geob::GeoType};

use crate::{
    error::Error,
    model::{
        Auto, Column, ForeignKey, Generated, Index, IndexPart, Key, Table, TableKind, TableName,
        Trigger, View,
    },
    rows::query,
};

type Result<T, E> = core::result::Result<T, Error<<E as Executor>::Connector>>;

/// By the name of the table
type Grouped<T> = HashMap<String, Vec<T>>;

pub async fn current_schema<E: Executor>(executor: &E) -> Result<String, E> {
    let rows = query(executor, "SELECT DATABASE() AS name", vec![]).await?;
    match rows.first() {
        Some(row) => row.opt("name")?.ok_or(Error::NoSchema),
        None => Err(Error::NoSchema),
    }
}

pub async fn schemas<E: Executor>(executor: &E) -> Result<Vec<String>, E> {
    query(
        executor,
        "SELECT SCHEMA_NAME AS name FROM information_schema.SCHEMATA
         WHERE SCHEMA_NAME NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys')
         ORDER BY CAST(SCHEMA_NAME AS BINARY)",
        vec![],
    )
    .await?
    .iter()
    .map(|row| row.text("name"))
    .collect()
}

async fn is_mariadb<E: Executor>(executor: &E) -> Result<bool, E> {
    let rows = query(executor, "SELECT VERSION() AS version", vec![]).await?;
    match rows.first() {
        Some(row) => Ok(row.text("version")?.contains("MariaDB")),
        None => Ok(false),
    }
}

/// The schema, and the name of a table twice
fn params(schema: &str, only: Option<&str>) -> Vec<Value> {
    let only = only.map_or(Value::Null, Value::from);
    vec![schema.into(), only.clone(), only]
}

const BASE_TABLES: &str = "('BASE TABLE', 'SYSTEM VERSIONED')";

fn kind(options: &str) -> TableKind {
    if options.to_ascii_lowercase().contains("partitioned") {
        TableKind::Partitioned
    } else {
        TableKind::Ordinary
    }
}

fn non_empty(text: Option<String>) -> Option<String> {
    text.filter(|m| !m.is_empty())
}

pub async fn names<E: Executor>(executor: &E, schema: &str) -> Result<Vec<TableName>, E> {
    let sql = format!(
        "SELECT TABLE_NAME AS name, CREATE_OPTIONS AS options FROM information_schema.TABLES
         WHERE TABLE_SCHEMA = ? AND (? IS NULL OR TABLE_NAME = ?) AND TABLE_TYPE IN {BASE_TABLES}
         ORDER BY CAST(TABLE_NAME AS BINARY)"
    );
    query(executor, &sql, params(schema, None))
        .await?
        .iter()
        .map(|row| {
            Ok(TableName {
                schema: schema.to_string(),
                name: row.text("name")?,
                kind: kind(&row.opt("options")?.unwrap_or_default()),
            })
        })
        .collect()
}

pub async fn tables<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Vec<Table>, E> {
    let sql = format!(
        "SELECT TABLE_NAME AS name, CREATE_OPTIONS AS options, TABLE_COMMENT AS comment
         FROM information_schema.TABLES
         WHERE TABLE_SCHEMA = ? AND (? IS NULL OR TABLE_NAME = ?) AND TABLE_TYPE IN {BASE_TABLES}
         ORDER BY CAST(TABLE_NAME AS BINARY)"
    );
    let rows = query(executor, &sql, params(schema, only)).await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let mariadb = is_mariadb(executor).await?;
    let mut columns = columns(executor, schema, only, mariadb).await?;
    let mut keys = constraints(executor, schema, only).await?;
    let mut indexes = indexes(executor, schema, only, mariadb).await?;
    let mut triggers = triggers(executor, schema, only).await?;

    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let name = row.text("name")?;
        let mut table = Table {
            system: executor.db_info().variant(),
            schema: schema.to_string(),
            kind: kind(&row.opt("options")?.unwrap_or_default()),
            comment: non_empty(row.opt("comment")?),
            partition_of: None,
            strict: false,
            without_rowid: false,
            columns: columns.remove(&name).unwrap_or_default(),
            primary_key: None,
            foreign_keys: Vec::new(),
            uniques: Vec::new(),
            indexes: Vec::new(),
            triggers: triggers.remove(&name).unwrap_or_default(),
            name,
        };
        for constraint in keys.remove(&table.name).unwrap_or_default() {
            match constraint {
                Constraint::Primary(key) => table.primary_key = Some(key),
                Constraint::Foreign(key) => table.foreign_keys.push(key),
            }
        }
        table.indexes = indexes
            .remove(&table.name)
            .unwrap_or_default()
            .into_iter()
            .filter(|index| !for_foreign_key(index, &table.foreign_keys))
            .collect();
        output.push(table);
    }
    Ok(output)
}

async fn columns<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
    mariadb: bool,
) -> Result<Grouped<Column>, E> {
    let sql = "SELECT TABLE_NAME AS relation, COLUMN_NAME AS name,
                      ORDINAL_POSITION AS position, COLUMN_TYPE AS type_name,
                      DATA_TYPE AS data_type, IS_NULLABLE = 'YES' AS nullable,
                      COLUMN_DEFAULT AS dflt, EXTRA AS extra,
                      GENERATION_EXPRESSION AS expression, COLUMN_COMMENT AS comment
               FROM information_schema.COLUMNS
               WHERE TABLE_SCHEMA = ? AND (? IS NULL OR TABLE_NAME = ?)
               ORDER BY ORDINAL_POSITION";

    let mut output = Grouped::new();
    for row in query(executor, sql, params(schema, only)).await? {
        let extra = row.opt("extra")?.unwrap_or_default().to_ascii_uppercase();
        let type_name = row.text("type_name")?;
        let data_type = row.text("data_type")?.to_ascii_lowercase();

        let generated = if extra.contains("GENERATED") && !extra.contains("DEFAULT_GENERATED") {
            let expression = non_empty(row.opt("expression")?);
            Some(Generated {
                expression: expression.map(|m| if mariadb { m } else { unescape(&m) }),
                stored: extra.contains("STORED"),
            })
        } else {
            None
        };
        let default = match generated {
            Some(_) => None,
            None => {
                let raw = row.opt("dflt")?;
                if mariadb {
                    raw.filter(|m| m != "NULL")
                } else {
                    raw.map(|raw| default_sql(raw, extra.contains("DEFAULT_GENERATED"), &data_type))
                }
            }
        };

        output
            .entry(row.text("relation")?)
            .or_default()
            .push(Column {
                name: row.text("name")?,
                position: row.int("position")?,
                ty: column_type(&data_type, &type_name),
                type_name,
                nullable: row.bool("nullable")?,
                default,
                auto: extra
                    .contains("AUTO_INCREMENT")
                    .then_some(Auto::AutoIncrement),
                generated,
                on_update: on_update(&extra),
                comment: non_empty(row.opt("comment")?),
            });
    }
    Ok(output)
}

/// From `on update CURRENT_TIMESTAMP(3)` in the extra information
fn on_update(extra: &str) -> Option<String> {
    let (_, rest) = extra.split_once("ON UPDATE ")?;
    let expression = rest.split_whitespace().next()?;
    Some(normalize_now(expression))
}

/// `now()` is how MySQL writes `(CURRENT_TIMESTAMP)`, which is how the builder writes it
fn normalize_now(sql: &str) -> String {
    let precision = sql
        .strip_prefix("now(")
        .and_then(|m| m.strip_suffix(')'))
        .filter(|m| m.bytes().all(|b| b.is_ascii_digit()));
    match precision {
        Some("") => "CURRENT_TIMESTAMP".to_string(),
        Some(precision) => format!("CURRENT_TIMESTAMP({precision})"),
        None => sql.to_string(),
    }
}

/// Undo the escaping of expressions in `information_schema`, as in `_utf8mb4\'x\'`
fn unescape(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(next) = chars.next() {
        match next {
            '\\' => output.extend(chars.next()),
            next => output.push(next),
        }
    }
    output
}

/// A literal is read without quotes, and an expression with escapes
fn default_sql(raw: String, expression: bool, data_type: &str) -> String {
    if expression {
        let sql = unescape(&raw);
        // The same as a literal, which is how it is written by the builder
        return match string_literal(&sql) {
            Some(value) => quote(&value),
            None => normalize_now(&sql),
        };
    }
    let numeric = matches!(
        data_type,
        "tinyint"
            | "smallint"
            | "mediumint"
            | "int"
            | "bigint"
            | "decimal"
            | "float"
            | "double"
            | "year"
    );
    let raw_sql = numeric
        || (data_type == "bit" && raw.starts_with("b'"))
        || (data_type.contains("binary") && raw.starts_with("0x"));
    if raw_sql { raw } else { quote(&raw) }
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "''"))
}

/// The value of a string literal, with an optional character set
fn string_literal(sql: &str) -> Option<String> {
    let rest = match sql.strip_prefix('_') {
        Some(rest) => rest.trim_start_matches(|m: char| m.is_ascii_alphanumeric()),
        None => sql,
    };
    let body = rest.strip_prefix('\'')?.strip_suffix('\'')?;
    let mut value = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(next) = chars.next() {
        match next {
            '\\' => value.push(match chars.next()? {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '0' => '\0',
                'b' => '\u{8}',
                'Z' => '\u{1a}',
                other => other,
            }),
            // Only a doubled quote is part of the value
            '\'' if chars.next_if_eq(&'\'').is_some() => value.push('\''),
            '\'' => return None,
            next => value.push(next),
        }
    }
    Some(value)
}

fn column_type(data_type: &str, type_name: &str) -> Option<Type> {
    let unsigned = type_name.contains("unsigned");
    Some(match data_type {
        "tinyint" if type_name.starts_with("tinyint(1)") => Type::Bool,
        "bool" | "boolean" => Type::Bool,
        "tinyint" | "year" => Type::SmallInt,
        "smallint" if unsigned => Type::Int,
        "smallint" => Type::SmallInt,
        "mediumint" => Type::Int,
        "int" | "integer" if unsigned => Type::BigInt,
        "int" | "integer" => Type::Int,
        "bigint" => Type::BigInt,
        "float" => Type::Float,
        "double" | "real" => Type::Double,
        "char" | "varchar" | "tinytext" | "text" | "mediumtext" | "longtext" | "enum" | "set"
        | "decimal" => Type::Text,
        "binary" | "varbinary" | "tinyblob" | "blob" | "mediumblob" | "longblob" | "bit" => {
            Type::Blob
        }
        "date" => Type::Date,
        "time" => Type::Time,
        "datetime" | "timestamp" => Type::DateTime,
        "json" => Type::Json,
        "point" => Type::Geometry(GeoType::Point),
        "linestring" => Type::Geometry(GeoType::LineString),
        "polygon" => Type::Geometry(GeoType::Polygon),
        "multipoint" => Type::Geometry(GeoType::MultiPoint),
        "multilinestring" => Type::Geometry(GeoType::MultiLineString),
        "multipolygon" => Type::Geometry(GeoType::MultiPolygon),
        "geometrycollection" | "geomcollection" => Type::Geometry(GeoType::Collection),
        _ => return None,
    })
}

enum Constraint {
    Primary(Key),
    Foreign(ForeignKey),
}

async fn constraints<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Grouped<Constraint>, E> {
    let sql = "SELECT c.TABLE_NAME AS relation, c.CONSTRAINT_NAME AS name,
                      c.CONSTRAINT_TYPE = 'PRIMARY KEY' AS is_primary, k.COLUMN_NAME AS col,
                      k.REFERENCED_TABLE_SCHEMA AS ref_schema,
                      k.REFERENCED_TABLE_NAME AS ref_table,
                      k.REFERENCED_COLUMN_NAME AS ref_column,
                      r.UPDATE_RULE AS on_update, r.DELETE_RULE AS on_delete
               FROM information_schema.TABLE_CONSTRAINTS c
               JOIN information_schema.KEY_COLUMN_USAGE k
                 ON k.CONSTRAINT_SCHEMA = c.CONSTRAINT_SCHEMA
                AND k.CONSTRAINT_NAME = c.CONSTRAINT_NAME AND k.TABLE_NAME = c.TABLE_NAME
               LEFT JOIN information_schema.REFERENTIAL_CONSTRAINTS r
                 ON r.CONSTRAINT_SCHEMA = c.CONSTRAINT_SCHEMA
                AND r.CONSTRAINT_NAME = c.CONSTRAINT_NAME AND r.TABLE_NAME = c.TABLE_NAME
               WHERE c.CONSTRAINT_SCHEMA = ? AND (? IS NULL OR c.TABLE_NAME = ?)
                 AND c.CONSTRAINT_TYPE IN ('PRIMARY KEY', 'FOREIGN KEY')
               ORDER BY CAST(c.TABLE_NAME AS BINARY), CAST(c.CONSTRAINT_NAME AS BINARY),
                        k.ORDINAL_POSITION";

    let mut output = Grouped::<Constraint>::new();
    let mut last: Option<(String, String)> = None;
    for row in query(executor, sql, params(schema, only)).await? {
        let relation = row.text("relation")?;
        let name = row.text("name")?;
        let column = row.text("col")?;
        let same = last.as_ref() == Some(&(relation.clone(), name.clone()));
        let entry = output.entry(relation.clone()).or_default();

        match entry.last_mut() {
            Some(Constraint::Primary(key)) if same => key.columns.push(column),
            Some(Constraint::Foreign(key)) if same => {
                key.columns.push(column);
                key.ref_columns.push(row.text("ref_column")?);
            }
            _ if row.bool("is_primary")? => entry.push(Constraint::Primary(Key {
                // Always PRIMARY
                name: None,
                columns: vec![column],
            })),
            _ => entry.push(Constraint::Foreign(ForeignKey {
                columns: vec![column],
                ref_schema: row.text("ref_schema")?,
                ref_table: row.text("ref_table")?,
                ref_columns: vec![row.text("ref_column")?],
                on_update: action(&row.text("on_update")?),
                on_delete: action(&row.text("on_delete")?),
                name: Some(name.clone()),
            })),
        }
        last = Some((relation, name));
    }
    Ok(output)
}

fn action(name: &str) -> ReferentialAction {
    match name {
        "CASCADE" => ReferentialAction::Cascade,
        "RESTRICT" => ReferentialAction::Restrict,
        "SET NULL" => ReferentialAction::SetNull,
        "SET DEFAULT" => ReferentialAction::SetDefault,
        _ => ReferentialAction::NoAction,
    }
}

/// Added by MySQL for the foreign key, and named after it or its first column
fn for_foreign_key(index: &Index, foreign_keys: &[ForeignKey]) -> bool {
    if index.unique {
        return false;
    }
    foreign_keys.iter().any(|fk| {
        let columns = index.parts.iter().map(|part| match part {
            IndexPart {
                column: Some(column),
                expression: None,
                descending: false,
                ..
            } => Some(column),
            _ => None,
        });
        let same = columns.eq(fk.columns.iter().map(Some));
        let named = fk.name.as_ref() == Some(&index.name)
            || fk.columns.first().is_some_and(|first| {
                index.name == *first
                    || index
                        .name
                        .strip_prefix(first.as_str())
                        .and_then(|m| m.strip_prefix('_'))
                        .is_some_and(|m| !m.is_empty() && m.bytes().all(|b| b.is_ascii_digit()))
            });
        same && named
    })
}

async fn indexes<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
    mariadb: bool,
) -> Result<Grouped<Index>, E> {
    // Indexes on expressions are new in MySQL 8.0.13, and not in MariaDB
    let expression = if mariadb { "NULL" } else { "EXPRESSION" };
    let sql = format!(
        "SELECT TABLE_NAME AS relation, INDEX_NAME AS name, NON_UNIQUE = 0 AS uniq,
                INDEX_TYPE AS method, COLUMN_NAME AS col, {expression} AS expression,
                COLLATION = 'D' AS descending, SUB_PART AS sub_part
         FROM information_schema.STATISTICS
         WHERE TABLE_SCHEMA = ? AND (? IS NULL OR TABLE_NAME = ?) AND INDEX_NAME <> 'PRIMARY'
         ORDER BY CAST(TABLE_NAME AS BINARY), CAST(INDEX_NAME AS BINARY), SEQ_IN_INDEX"
    );

    let mut output = Grouped::<Index>::new();
    for row in query(executor, &sql, params(schema, only)).await? {
        let relation = row.text("relation")?;
        let name = row.text("name")?;
        let column = row.opt("col")?;
        let descending = row.opt_bool("descending")?.unwrap_or_default();
        let part = match (column, row.opt_int("sub_part")?) {
            // Only a prefix of the column, as `name`(10)
            (Some(column), Some(length)) => IndexPart {
                expression: Some(format!("`{}`({length})", column.replace('`', "``"))),
                column: None,
                descending,
                collation: None,
            },
            (column, _) => IndexPart {
                expression: match column {
                    Some(_) => None,
                    None => row.opt("expression")?.map(|m| unescape(&m)),
                },
                column,
                descending,
                collation: None,
            },
        };

        let entry = output.entry(relation).or_default();
        match entry.last_mut() {
            Some(index) if index.name == name => index.parts.push(part),
            _ => entry.push(Index {
                unique: row.bool("uniq")?,
                method: row.text("method")?.to_ascii_lowercase(),
                parts: vec![part],
                included: Vec::new(),
                partial: false,
                predicate: None,
                definition: None,
                name,
            }),
        }
    }
    Ok(output)
}

fn identifier(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

async fn triggers<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Grouped<Trigger>, E> {
    let sql = "SELECT EVENT_OBJECT_TABLE AS relation, TRIGGER_NAME AS name,
                      ACTION_TIMING AS timing, EVENT_MANIPULATION AS event,
                      ACTION_STATEMENT AS statement
               FROM information_schema.TRIGGERS
               WHERE TRIGGER_SCHEMA = ? AND (? IS NULL OR EVENT_OBJECT_TABLE = ?)
               ORDER BY CAST(TRIGGER_NAME AS BINARY)";
    let mut output = Grouped::new();
    for row in query(executor, sql, params(schema, only)).await? {
        let relation = row.text("relation")?;
        let name = row.text("name")?;
        let definition = format!(
            "CREATE TRIGGER {} {} {} ON {} FOR EACH ROW {}",
            identifier(&name),
            row.text("timing")?,
            row.text("event")?,
            identifier(&relation),
            row.text("statement")?,
        );
        output
            .entry(relation)
            .or_default()
            .push(Trigger { name, definition });
    }
    Ok(output)
}

pub async fn views<E: Executor>(executor: &E, schema: &str) -> Result<Vec<View>, E> {
    let rows = query(
        executor,
        "SELECT TABLE_NAME AS name, VIEW_DEFINITION AS definition
         FROM information_schema.VIEWS WHERE TABLE_SCHEMA = ?
         ORDER BY CAST(TABLE_NAME AS BINARY)",
        vec![schema.into()],
    )
    .await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let mariadb = is_mariadb(executor).await?;
    let mut columns = columns(executor, schema, None, mariadb).await?;
    rows.iter()
        .map(|row| {
            let name = row.text("name")?;
            Ok(View {
                schema: schema.to_string(),
                materialized: false,
                comment: None,
                columns: columns.remove(&name).unwrap_or_default(),
                triggers: Vec::new(),
                definition: non_empty(row.opt("definition")?),
                name,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        for (raw, expression, data_type, expected) in [
            ("it's", false, "varchar", "'it''s'"),
            ("a\\b", false, "text", "'a\\\\b'"),
            ("5", false, "int", "5"),
            ("1.50", false, "decimal", "1.50"),
            ("0x0102", false, "binary", "0x0102"),
            ("b'1'", false, "bit", "b'1'"),
            (
                "2020-01-01 00:00:00",
                false,
                "datetime",
                "'2020-01-01 00:00:00'",
            ),
            ("(1 + 2)", true, "int", "(1 + 2)"),
            ("CURRENT_TIMESTAMP", true, "datetime", "CURRENT_TIMESTAMP"),
            ("now()", true, "datetime", "CURRENT_TIMESTAMP"),
            ("now(6)", true, "datetime", "CURRENT_TIMESTAMP(6)"),
            ("now(a)", true, "datetime", "now(a)"),
            (r"_latin1\'it\\\'s\'", true, "varchar", "'it''s'"),
            (r"_utf8mb4\'a\\\\b\'", true, "varchar", "'a\\\\b'"),
            (
                r"concat(_utf8mb4\'a\',_utf8mb4\'b\')",
                true,
                "varchar",
                "concat(_utf8mb4'a',_utf8mb4'b')",
            ),
        ] {
            assert_eq!(
                default_sql(raw.to_string(), expression, data_type),
                expected,
                "{raw}"
            );
        }
    }

    #[test]
    fn on_updates() {
        let extra = "DEFAULT_GENERATED ON UPDATE CURRENT_TIMESTAMP(3)";
        assert_eq!(on_update(extra).as_deref(), Some("CURRENT_TIMESTAMP(3)"));
        assert_eq!(on_update("DEFAULT_GENERATED"), None);
    }

    #[test]
    fn literals() {
        assert_eq!(string_literal("'a''b'").as_deref(), Some("a'b"));
        assert_eq!(string_literal("_utf8mb4'a\\nb'").as_deref(), Some("a\nb"));
        assert_eq!(string_literal("'a' || 'b'"), None);
        assert_eq!(string_literal("concat('a')"), None);
    }

    #[test]
    fn types() {
        for (data_type, type_name, ty) in [
            ("tinyint", "tinyint(1)", Some(Type::Bool)),
            ("tinyint", "tinyint", Some(Type::SmallInt)),
            ("int", "int unsigned", Some(Type::BigInt)),
            ("int", "int", Some(Type::Int)),
            ("decimal", "decimal(10,2)", Some(Type::Text)),
            ("enum", "enum('a','b')", Some(Type::Text)),
            ("binary", "binary(16)", Some(Type::Blob)),
            ("point", "point", Some(Type::Geometry(GeoType::Point))),
            ("geometry", "geometry", None),
        ] {
            assert_eq!(column_type(data_type, type_name), ty, "{type_name}");
        }
    }
}
