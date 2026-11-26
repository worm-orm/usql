use std::collections::BTreeMap;

use usql_builder::schema::ReferentialAction;
use usql_core::{DatabaseInfo, Executor};
use usql_value::{Type, Value};

use crate::{
    error::Error,
    model::{
        Auto, Column, ForeignKey, Generated, Index, IndexPart, Key, Table, TableKind, TableName,
        Trigger, View,
    },
    rows::query,
};

type Result<T, E> = core::result::Result<T, Error<<E as Executor>::Connector>>;

fn params(values: &[&str]) -> Vec<Value> {
    values.iter().map(|m| Value::from(*m)).collect()
}

pub async fn schemas<E: Executor>(executor: &E) -> Result<Vec<String>, E> {
    query(
        executor,
        "SELECT name FROM pragma_database_list ORDER BY seq",
        vec![],
    )
    .await?
    .iter()
    .map(|row| row.text("name"))
    .collect()
}

struct Info {
    name: String,
    kind: TableKind,
    without_rowid: bool,
    strict: bool,
}

async fn list<E: Executor>(executor: &E, schema: &str) -> Result<Vec<Info>, E> {
    query(
        executor,
        r#"SELECT name, type, wr, strict FROM pragma_table_list
           WHERE "schema" = ? AND type IN ('table', 'virtual')
             AND lower(substr(name, 1, 7)) <> 'sqlite_'
           ORDER BY name"#,
        params(&[schema]),
    )
    .await?
    .iter()
    .map(|row| {
        Ok(Info {
            name: row.text("name")?,
            kind: match row.text("type")?.as_str() {
                "virtual" => TableKind::Virtual,
                _ => TableKind::Ordinary,
            },
            without_rowid: row.bool("wr")?,
            strict: row.bool("strict")?,
        })
    })
    .collect()
}

pub async fn names<E: Executor>(executor: &E, schema: &str) -> Result<Vec<TableName>, E> {
    let names = list(executor, schema)
        .await?
        .into_iter()
        .map(|info| TableName {
            schema: schema.to_string(),
            name: info.name,
            kind: info.kind,
        })
        .collect();
    Ok(names)
}

/// Objects of a type in `sqlite_schema`
struct Object {
    name: String,
    table: String,
    sql: Option<String>,
}

async fn objects<E: Executor>(
    executor: &E,
    schema: &str,
) -> Result<BTreeMap<String, Vec<Object>>, E> {
    // The schema is an identifier, and can not be bound
    let sql = format!(
        "SELECT type, name, tbl_name, sql FROM \"{}\".sqlite_schema ORDER BY name",
        schema.replace('"', "\"\"")
    );
    let mut output = BTreeMap::<String, Vec<Object>>::new();
    for row in query(executor, &sql, vec![]).await? {
        output.entry(row.text("type")?).or_default().push(Object {
            name: row.text("name")?,
            table: row.text("tbl_name")?,
            sql: row.opt("sql")?,
        });
    }
    Ok(output)
}

fn find<'a>(objects: &'a BTreeMap<String, Vec<Object>>, ty: &str, name: &str) -> Option<&'a str> {
    objects
        .get(ty)?
        .iter()
        .find(|m| m.name == name)?
        .sql
        .as_deref()
}

fn triggers(objects: &BTreeMap<String, Vec<Object>>, table: &str) -> Vec<Trigger> {
    objects
        .get("trigger")
        .into_iter()
        .flatten()
        .filter(|m| m.table.eq_ignore_ascii_case(table))
        .filter_map(|m| {
            Some(Trigger {
                name: m.name.clone(),
                definition: m.sql.clone()?,
            })
        })
        .collect()
}

pub async fn tables<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Vec<Table>, E> {
    let infos = list(executor, schema).await?;
    if infos.is_empty() {
        return Ok(Vec::new());
    }
    let objects = objects(executor, schema).await?;
    let mut output = Vec::new();

    for info in infos {
        // As names are in SQLite
        if only.is_some_and(|only| !only.eq_ignore_ascii_case(&info.name)) {
            continue;
        }

        let (mut columns, keys) = columns(executor, schema, &info.name).await?;

        // An alias of the rowid, which is never null
        if let [key] = keys.as_slice()
            && !info.without_rowid
            && let Some(column) = columns.iter_mut().find(|m| m.name == *key)
            && column.type_name.eq_ignore_ascii_case("INTEGER")
        {
            column.nullable = false;
            let sql = find(&objects, "table", &info.name);
            if sql.is_some_and(is_autoincrement) {
                column.auto = Some(Auto::AutoIncrement);
            }
        }

        let (uniques, indexes) = indexes(executor, schema, &info.name, &objects).await?;

        output.push(Table {
            system: executor.db_info().variant(),
            schema: schema.to_string(),
            kind: info.kind,
            comment: None,
            partition_of: None,
            strict: info.strict,
            without_rowid: info.without_rowid,
            columns,
            primary_key: (!keys.is_empty()).then_some(Key {
                name: None,
                columns: keys,
            }),
            foreign_keys: foreign_keys(executor, schema, &info.name).await?,
            uniques,
            indexes,
            triggers: triggers(&objects, &info.name),
            name: info.name,
        });
    }

    Ok(output)
}

/// The columns, and the names of those in the primary key
async fn columns<E: Executor>(
    executor: &E,
    schema: &str,
    table: &str,
) -> Result<(Vec<Column>, Vec<String>), E> {
    let rows = query(
        executor,
        r#"SELECT cid, name, type, "notnull" AS required, dflt_value, pk, hidden
           FROM pragma_table_xinfo(?, ?) ORDER BY cid"#,
        params(&[table, schema]),
    )
    .await?;

    let mut columns = Vec::new();
    let mut keys = Vec::new();

    for row in rows {
        let name = row.text("name")?;
        let hidden = row.int("hidden")?;
        // A hidden column of a virtual table
        if hidden == 1 {
            continue;
        }
        let pk = row.int("pk")?;
        if pk > 0 {
            keys.push((pk, name.clone()));
        }
        let type_name = row.text("type")?;
        columns.push(Column {
            name,
            position: row.int("cid")? + 1,
            ty: column_type(&type_name),
            type_name,
            nullable: !row.bool("required")?,
            // An explicit null is the same as none
            default: row
                .opt("dflt_value")?
                .filter(|m| !m.eq_ignore_ascii_case("NULL")),
            auto: None,
            generated: matches!(hidden, 2 | 3).then_some(Generated {
                expression: None,
                stored: hidden == 3,
            }),
            on_update: None,
            comment: None,
        });
    }

    keys.sort();
    Ok((columns, keys.into_iter().map(|m| m.1).collect()))
}

fn column_type(name: &str) -> Option<Type> {
    let name = name.split('(').next().unwrap_or("").trim();
    let name = name.to_ascii_uppercase();
    Some(match name.as_str() {
        "BOOL" | "BOOLEAN" => Type::Bool,
        "DATE" => Type::Date,
        "TIME" => Type::Time,
        "DATETIME" | "TIMESTAMP" => Type::DateTime,
        "UUID" => Type::Uuid,
        "JSON" => Type::Json,
        "POINT" => return None,
        // The affinities
        name if name.contains("INT") => Type::BigInt,
        name if ["CHAR", "CLOB", "TEXT"].iter().any(|m| name.contains(m)) => Type::Text,
        name if name.contains("BLOB") => Type::Blob,
        name if ["REAL", "FLOA", "DOUB"].iter().any(|m| name.contains(m)) => Type::Double,
        _ => return None,
    })
}

async fn indexes<E: Executor>(
    executor: &E,
    schema: &str,
    table: &str,
    objects: &BTreeMap<String, Vec<Object>>,
) -> Result<(Vec<Key>, Vec<Index>), E> {
    let mut uniques = Vec::new();
    let mut indexes = Vec::new();

    let rows = query(
        executor,
        r#"SELECT name, "unique" AS uniq, origin, partial
           FROM pragma_index_list(?, ?) ORDER BY name"#,
        params(&[table, schema]),
    )
    .await?;

    for row in rows {
        let origin = row.text("origin")?;
        if origin == "pk" {
            continue;
        }
        let name = row.text("name")?;
        let parts = query(
            executor,
            r#"SELECT cid, name, "desc" AS descending, coll
               FROM pragma_index_xinfo(?, ?) WHERE key = 1 ORDER BY seqno"#,
            params(&[&name, schema]),
        )
        .await?
        .iter()
        .map(|row| {
            Ok(IndexPart {
                column: match row.int("cid")? {
                    -2 => None,
                    -1 => Some("rowid".to_string()),
                    _ => Some(row.text("name")?),
                },
                expression: None,
                descending: row.bool("descending")?,
                collation: row
                    .opt("coll")?
                    .filter(|m| !m.eq_ignore_ascii_case("BINARY")),
            })
        })
        .collect::<Result<Vec<_>, E>>()?;

        if origin == "u" {
            uniques.push(Key {
                name: None,
                columns: parts.into_iter().filter_map(|m| m.column).collect(),
            });
        } else {
            indexes.push(Index {
                definition: find(objects, "index", &name).map(ToString::to_string),
                name,
                unique: row.bool("uniq")?,
                method: "btree".to_string(),
                parts,
                included: Vec::new(),
                partial: row.bool("partial")?,
                predicate: None,
            });
        }
    }

    Ok((uniques, indexes))
}

async fn foreign_keys<E: Executor>(
    executor: &E,
    schema: &str,
    table: &str,
) -> Result<Vec<ForeignKey>, E> {
    // The last one declared is listed first
    let rows = query(
        executor,
        r#"SELECT id, "table" AS ref_table, "from" AS col, "to" AS ref_col,
                  on_update, on_delete
           FROM pragma_foreign_key_list(?, ?) ORDER BY id DESC, seq"#,
        params(&[table, schema]),
    )
    .await?;

    let mut output: Vec<(i32, ForeignKey)> = Vec::new();

    for row in rows {
        let id = row.int("id")?;
        if output.last().is_none_or(|m| m.0 != id) {
            output.push((
                id,
                ForeignKey {
                    name: None,
                    columns: Vec::new(),
                    ref_schema: schema.to_string(),
                    ref_table: row.text("ref_table")?,
                    ref_columns: Vec::new(),
                    on_update: action(&row.text("on_update")?),
                    on_delete: action(&row.text("on_delete")?),
                },
            ));
        }
        if let Some((_, fk)) = output.last_mut() {
            fk.columns.push(row.text("col")?);
            fk.ref_columns.extend(row.opt("ref_col")?);
        }
    }

    let mut keys = Vec::with_capacity(output.len());
    for (_, mut fk) in output {
        // Refers to the primary key when there are no columns
        if fk.ref_columns.is_empty() {
            fk.ref_columns = columns(executor, schema, &fk.ref_table).await?.1;
        }
        keys.push(fk);
    }
    Ok(keys)
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

pub async fn views<E: Executor>(executor: &E, schema: &str) -> Result<Vec<View>, E> {
    let objects = objects(executor, schema).await?;
    let mut output = Vec::new();
    for view in objects.get("view").into_iter().flatten() {
        output.push(View {
            schema: schema.to_string(),
            name: view.name.clone(),
            materialized: false,
            comment: None,
            columns: columns(executor, schema, &view.name).await?.0,
            triggers: triggers(&objects, &view.name),
            definition: view.sql.clone(),
        });
    }
    Ok(output)
}

/// Is `AUTOINCREMENT` a keyword in the statement
fn is_autoincrement(sql: &str) -> bool {
    let mut rest = sql;
    while let Some(ch) = rest.chars().next() {
        rest = match ch {
            '\'' | '"' | '`' | '[' => {
                let end = if ch == '[' { ']' } else { ch };
                // An escaped quote is seen as two strings
                rest[1..].split_once(end).map_or("", |m| m.1)
            }
            '-' if rest.starts_with("--") => rest.split_once('\n').map_or("", |m| m.1),
            '/' if rest.starts_with("/*") => rest[2..].split_once("*/").map_or("", |m| m.1),
            ch if is_word(ch) => {
                let end = rest.find(|m| !is_word(m)).unwrap_or(rest.len());
                let (word, next) = rest.split_at(end);
                if word.eq_ignore_ascii_case("AUTOINCREMENT") {
                    return true;
                }
                next
            }
            ch => &rest[ch.len_utf8()..],
        };
    }
    false
}

fn is_word(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' || !ch.is_ascii()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoincrement() {
        assert!(is_autoincrement(
            "CREATE TABLE t(id INTEGER PRIMARY KEY autoincrement)"
        ));
        assert!(is_autoincrement(
            "CREATE TABLE \"it''s\"(id INTEGER PRIMARY KEY/**/AUTOINCREMENT)"
        ));
        for sql in [
            "CREATE TABLE t(id INTEGER PRIMARY KEY -- AUTOINCREMENT\n)",
            "CREATE TABLE t(id INTEGER /* AUTOINCREMENT */)",
            "CREATE TABLE t(a DEFAULT 'it''s AUTOINCREMENT')",
            "CREATE TABLE t(\"AUTOINCREMENT\", [AUTOINCREMENT], `AUTOINCREMENT`)",
            "CREATE TABLE t(my_autoincrement, øautoincrement)",
            "CREATE TABLE t(a 'unterminated AUTOINCREMENT",
        ] {
            assert!(!is_autoincrement(sql), "{sql}");
        }
    }
}
