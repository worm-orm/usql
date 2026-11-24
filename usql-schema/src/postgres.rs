use std::collections::HashMap;

use usql_builder::schema::ReferentialAction;
use usql_core::{DatabaseInfo, Executor};
use usql_value::{Type, Value, geob::GeoType};

use crate::{
    error::Error,
    model::{
        Auto, Column, Enum, ForeignKey, Generated, Index, IndexPart, Key, Table, TableKind,
        TableName, Trigger, View,
    },
    rows::query,
};

type Result<T, E> = core::result::Result<T, Error<<E as Executor>::Connector>>;

pub async fn current_schema<E: Executor>(executor: &E) -> Result<String, E> {
    let rows = query(executor, "SELECT current_schema()::text AS name", vec![]).await?;
    match rows.first() {
        Some(row) => row.opt("name")?.ok_or(Error::NoSchema),
        None => Err(Error::NoSchema),
    }
}

pub async fn schemas<E: Executor>(executor: &E) -> Result<Vec<String>, E> {
    query(
        executor,
        r#"SELECT nspname::text AS name FROM pg_catalog.pg_namespace
           WHERE nspname !~ '^pg_' AND nspname <> 'information_schema'
           ORDER BY nspname COLLATE "C""#,
        vec![],
    )
    .await?
    .iter()
    .map(|row| row.text("name"))
    .collect()
}

fn params(schema: &str, only: Option<&str>) -> Vec<Value> {
    vec![schema.into(), only.map_or(Value::Null, Value::from)]
}

/// Relations in the schema, that are not part of an extension
const RELATIONS: &str = r#"
    FROM pg_catalog.pg_class c
    JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace AND n.nspname = $1
    WHERE ($2::text IS NULL OR c.relname = $2::text)
      AND NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_depend e
        WHERE e.classid = 'pg_catalog.pg_class'::regclass AND e.objid = c.oid AND e.deptype = 'e'
      )"#;

fn kind(relkind: &str) -> TableKind {
    match relkind {
        "p" => TableKind::Partitioned,
        "f" => TableKind::Foreign,
        _ => TableKind::Ordinary,
    }
}

pub async fn names<E: Executor>(executor: &E, schema: &str) -> Result<Vec<TableName>, E> {
    let sql = format!(
        r#"SELECT c.relname::text AS name, c.relkind::text AS kind {RELATIONS}
           AND c.relkind IN ('r', 'p', 'f') ORDER BY c.relname COLLATE "C""#
    );
    query(executor, &sql, params(schema, None))
        .await?
        .iter()
        .map(|row| {
            Ok(TableName {
                schema: schema.to_string(),
                name: row.text("name")?,
                kind: kind(&row.text("kind")?),
            })
        })
        .collect()
}

/// By the name of the relation
type Grouped<T> = HashMap<String, Vec<T>>;

pub async fn tables<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Vec<Table>, E> {
    let sql = format!(
        r#"SELECT c.relname::text AS name, c.relkind::text AS kind,
                  pg_catalog.obj_description(c.oid, 'pg_class') AS comment,
                  (SELECT p.relname::text FROM pg_catalog.pg_inherits i
                   JOIN pg_catalog.pg_class p ON p.oid = i.inhparent
                   WHERE i.inhrelid = c.oid AND c.relispartition) AS partition_of
           {RELATIONS} AND c.relkind IN ('r', 'p', 'f') ORDER BY c.relname COLLATE "C""#
    );
    let rows = query(executor, &sql, params(schema, only)).await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut columns = columns(executor, schema, only).await?;
    let mut keys = constraints(executor, schema, only).await?;
    let mut indexes = indexes(executor, schema, only).await?;
    let mut triggers = triggers(executor, schema, only).await?;

    let mut output = Vec::with_capacity(rows.len());
    for row in rows {
        let name = row.text("name")?;
        let mut table = Table {
            system: executor.db_info().variant(),
            schema: schema.to_string(),
            kind: kind(&row.text("kind")?),
            comment: row.opt("comment")?,
            partition_of: row.opt("partition_of")?,
            strict: false,
            without_rowid: false,
            columns: columns.remove(&name).unwrap_or_default(),
            primary_key: None,
            foreign_keys: Vec::new(),
            uniques: Vec::new(),
            indexes: indexes.remove(&name).unwrap_or_default(),
            triggers: triggers.remove(&name).unwrap_or_default(),
            name,
        };
        for constraint in keys.remove(&table.name).unwrap_or_default() {
            match constraint {
                Constraint::Primary(key) => table.primary_key = Some(key),
                Constraint::Unique(key) => table.uniques.push(key),
                Constraint::Foreign(key) => table.foreign_keys.push(key),
            }
        }
        output.push(table);
    }
    Ok(output)
}

async fn columns<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Grouped<Column>, E> {
    let sql = format!(
        r#"SELECT r.name AS relation, a.attname::text AS name, a.attnum::int4 AS position,
                  pg_catalog.format_type(a.atttypid, a.atttypmod) AS type_name,
                  CASE t.typtype
                    WHEN 'd' THEN pg_catalog.format_type(t.typbasetype, t.typtypmod)
                    WHEN 'e' THEN 'text'
                    ELSE pg_catalog.format_type(a.atttypid, a.atttypmod)
                  END AS base,
                  t.typtype = 'd' AS is_domain,
                  a.attnotnull AS required,
                  pg_catalog.pg_get_expr(d.adbin, d.adrelid) AS expression,
                  a.attidentity::text AS identity, a.attgenerated::text AS generated,
                  EXISTS (
                    SELECT 1 FROM pg_catalog.pg_depend o
                    JOIN pg_catalog.pg_class s ON s.oid = o.objid AND s.relkind = 'S'
                    WHERE o.classid = 'pg_catalog.pg_class'::regclass AND o.deptype = 'a'
                      AND o.refclassid = 'pg_catalog.pg_class'::regclass
                      AND o.refobjid = r.oid AND o.refobjsubid = a.attnum
                  ) AS owns_sequence,
                  pg_catalog.col_description(r.oid, a.attnum) AS comment
           FROM (SELECT c.oid, c.relname::text AS name {RELATIONS}
                 AND c.relkind IN ('r', 'p', 'f', 'v', 'm')) r
           JOIN pg_catalog.pg_attribute a ON a.attrelid = r.oid
             AND a.attnum > 0 AND NOT a.attisdropped
           JOIN pg_catalog.pg_type t ON t.oid = a.atttypid
           LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = r.oid AND d.adnum = a.attnum
           ORDER BY a.attnum"#
    );

    let mut output = Grouped::new();
    for row in query(executor, &sql, params(schema, only)).await? {
        let generated = row.text("generated")?;
        let expression = row.opt("expression")?;
        let serial = row.bool("owns_sequence")?
            && expression
                .as_deref()
                .is_some_and(|m| m.starts_with("nextval("));

        let auto = if !row.text("identity")?.is_empty() {
            Some(Auto::Identity)
        } else if serial {
            Some(Auto::Serial)
        } else {
            None
        };

        let (default, generated) = if !generated.is_empty() {
            let generated = Generated {
                expression,
                stored: generated == "s",
            };
            (None, Some(generated))
        } else if serial {
            (None, None)
        } else {
            // An explicit null is the same as none, unless it replaces that of a domain
            let is_domain = row.bool("is_domain")?;
            (expression.filter(|m| is_domain || !is_null(m)), None)
        };

        output
            .entry(row.text("relation")?)
            .or_default()
            .push(Column {
                name: row.text("name")?,
                position: row.int("position")?,
                type_name: row.text("type_name")?,
                ty: column_type(&row.text("base")?),
                nullable: !row.bool("required")?,
                default,
                auto,
                generated,
                comment: row.opt("comment")?,
            });
    }
    Ok(output)
}

/// As `NULL::character varying`
fn is_null(sql: &str) -> bool {
    match sql.strip_prefix("NULL") {
        Some("") => true,
        Some(rest) => rest.strip_prefix("::").is_some_and(|ty| {
            ty.chars()
                .all(|m| m.is_alphanumeric() || " _.,()[]\"".contains(m))
        }),
        None => false,
    }
}

fn column_type(name: &str) -> Option<Type> {
    if let Some(item) = name.strip_suffix("[]") {
        return Some(Type::Array(Box::new(column_type(item)?)));
    }
    // As in "character varying(10)" and "timestamp(3) without time zone"
    let (name, args) = match name.split_once('(') {
        Some((name, rest)) => {
            let (args, suffix) = rest.split_once(')')?;
            (format!("{name}{suffix}"), Some(args))
        }
        None => (name.to_string(), None),
    };
    // Types that are not in the search path have a schema
    let name = name.rsplit('.').next().unwrap_or_default();
    Some(match name {
        "boolean" => Type::Bool,
        "smallint" => Type::SmallInt,
        "integer" => Type::Int,
        "bigint" | "oid" => Type::BigInt,
        "real" => Type::Float,
        "double precision" => Type::Double,
        "text" | "character varying" | "character" | "name" | "xml" | "citext" | "numeric" => {
            Type::Text
        }
        "bytea" => Type::Blob,
        "date" => Type::Date,
        "time without time zone" => Type::Time,
        "timestamp without time zone" | "timestamp with time zone" => Type::DateTime,
        "uuid" => Type::Uuid,
        "json" | "jsonb" => Type::Json,
        "geometry" | "geography" => {
            let ty = args?.split(',').next()?.to_ascii_lowercase();
            Type::Geometry(match ty.trim_end_matches(['z', 'm']) {
                "point" => GeoType::Point,
                "linestring" => GeoType::LineString,
                "polygon" => GeoType::Polygon,
                "multipoint" => GeoType::MultiPoint,
                "multilinestring" => GeoType::MultiLineString,
                "multipolygon" => GeoType::MultiPolygon,
                "geometrycollection" => GeoType::Collection,
                _ => return None,
            })
        }
        _ => return None,
    })
}

enum Constraint {
    Primary(Key),
    Unique(Key),
    Foreign(ForeignKey),
}

async fn constraints<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Grouped<Constraint>, E> {
    // A key to a partitioned table also has a constraint for each of its partitions
    let sql = format!(
        r#"SELECT r.name AS relation, k.conname::text AS name, k.contype::text AS kind,
                  ARRAY(SELECT a.attname::text
                        FROM unnest(k.conkey) WITH ORDINALITY u(num, pos)
                        JOIN pg_catalog.pg_attribute a
                          ON a.attrelid = k.conrelid AND a.attnum = u.num
                        ORDER BY u.pos) AS columns,
                  ARRAY(SELECT a.attname::text
                        FROM unnest(k.confkey) WITH ORDINALITY u(num, pos)
                        JOIN pg_catalog.pg_attribute a
                          ON a.attrelid = k.confrelid AND a.attnum = u.num
                        ORDER BY u.pos) AS ref_columns,
                  fn.nspname::text AS ref_schema, f.relname::text AS ref_table,
                  k.confupdtype::text AS on_update, k.confdeltype::text AS on_delete
           FROM (SELECT c.oid, c.relname::text AS name {RELATIONS}
                 AND c.relkind IN ('r', 'p', 'f')) r
           JOIN pg_catalog.pg_constraint k ON k.conrelid = r.oid
             AND k.contype IN ('p', 'u', 'f')
           LEFT JOIN pg_catalog.pg_class f ON f.oid = k.confrelid
           LEFT JOIN pg_catalog.pg_namespace fn ON fn.oid = f.relnamespace
           WHERE NOT EXISTS (
             SELECT 1 FROM pg_catalog.pg_constraint p
             WHERE p.oid = k.conparentid AND p.conrelid = k.conrelid
           )
           ORDER BY k.conname COLLATE "C""#
    );

    let mut output = Grouped::new();
    for row in query(executor, &sql, params(schema, only)).await? {
        let name = Some(row.text("name")?);
        let columns = row.texts("columns")?;
        let constraint = match row.text("kind")?.as_str() {
            "p" => Constraint::Primary(Key { name, columns }),
            "u" => Constraint::Unique(Key { name, columns }),
            _ => Constraint::Foreign(ForeignKey {
                name,
                columns,
                ref_schema: row.text("ref_schema")?,
                ref_table: row.text("ref_table")?,
                ref_columns: row.texts("ref_columns")?,
                on_update: action(&row.text("on_update")?),
                on_delete: action(&row.text("on_delete")?),
            }),
        };
        output
            .entry(row.text("relation")?)
            .or_default()
            .push(constraint);
    }
    Ok(output)
}

fn action(name: &str) -> ReferentialAction {
    match name {
        "c" => ReferentialAction::Cascade,
        "r" => ReferentialAction::Restrict,
        "n" => ReferentialAction::SetNull,
        "d" => ReferentialAction::SetDefault,
        _ => ReferentialAction::NoAction,
    }
}

async fn indexes<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Grouped<Index>, E> {
    let sql = format!(
        r#"SELECT r.name AS relation, ic.relname::text AS name, i.indisunique AS uniq,
                  am.amname::text AS method,
                  pg_catalog.pg_get_indexdef(i.indexrelid) AS definition,
                  pg_catalog.pg_get_expr(i.indpred, i.indrelid) AS predicate,
                  ARRAY(SELECT a.attname::text FROM generate_series(1, i.indnkeyatts) g(pos)
                        LEFT JOIN pg_catalog.pg_attribute a
                          ON a.attrelid = i.indrelid AND a.attnum = i.indkey[g.pos - 1]
                             AND a.attnum <> 0
                        ORDER BY g.pos) AS columns,
                  ARRAY(SELECT pg_catalog.pg_get_indexdef(i.indexrelid, g.pos, true)
                        FROM generate_series(1, i.indnkeyatts) g(pos)
                        ORDER BY g.pos) AS expressions,
                  ARRAY(SELECT (i.indoption[g.pos - 1] & 1) = 1
                        FROM generate_series(1, i.indnkeyatts) g(pos)
                        ORDER BY g.pos) AS descending,
                  ARRAY(SELECT a.attname::text
                        FROM generate_series(i.indnkeyatts + 1, i.indnatts) g(pos)
                        JOIN pg_catalog.pg_attribute a
                          ON a.attrelid = i.indrelid AND a.attnum = i.indkey[g.pos - 1]
                        ORDER BY g.pos) AS included
           FROM (SELECT c.oid, c.relname::text AS name {RELATIONS}
                 AND c.relkind IN ('r', 'p')) r
           JOIN pg_catalog.pg_index i ON i.indrelid = r.oid
           JOIN pg_catalog.pg_class ic ON ic.oid = i.indexrelid
           JOIN pg_catalog.pg_am am ON am.oid = ic.relam
           WHERE NOT EXISTS (
             SELECT 1 FROM pg_catalog.pg_constraint k
             WHERE k.conindid = i.indexrelid AND k.conrelid = i.indrelid
               AND k.contype IN ('p', 'u')
           )
           ORDER BY ic.relname COLLATE "C""#
    );

    let mut output = Grouped::new();
    for row in query(executor, &sql, params(schema, only)).await? {
        let parts = row
            .list("columns")?
            .into_iter()
            .zip(row.texts("expressions")?)
            .zip(row.bools("descending")?)
            .map(|((column, expression), descending)| IndexPart {
                expression: column.is_none().then_some(expression),
                column,
                descending,
                collation: None,
            })
            .collect();
        let predicate = row.opt("predicate")?;
        output
            .entry(row.text("relation")?)
            .or_default()
            .push(Index {
                name: row.text("name")?,
                unique: row.bool("uniq")?,
                method: row.text("method")?,
                parts,
                included: row.texts("included")?,
                partial: predicate.is_some(),
                predicate,
                definition: row.opt("definition")?,
            });
    }
    Ok(output)
}

async fn triggers<E: Executor>(
    executor: &E,
    schema: &str,
    only: Option<&str>,
) -> Result<Grouped<Trigger>, E> {
    let sql = format!(
        r#"SELECT r.name AS relation, t.tgname::text AS name,
                  pg_catalog.pg_get_triggerdef(t.oid) AS definition
           FROM (SELECT c.oid, c.relname::text AS name {RELATIONS}) r
           JOIN pg_catalog.pg_trigger t ON t.tgrelid = r.oid AND NOT t.tgisinternal
           ORDER BY t.tgname COLLATE "C""#
    );
    let mut output = Grouped::new();
    for row in query(executor, &sql, params(schema, only)).await? {
        output
            .entry(row.text("relation")?)
            .or_default()
            .push(Trigger {
                name: row.text("name")?,
                definition: row.text("definition")?,
            });
    }
    Ok(output)
}

pub async fn views<E: Executor>(executor: &E, schema: &str) -> Result<Vec<View>, E> {
    let sql = format!(
        r#"SELECT c.relname::text AS name, c.relkind = 'm' AS materialized,
                  pg_catalog.obj_description(c.oid, 'pg_class') AS comment,
                  pg_catalog.pg_get_viewdef(c.oid) AS definition
           {RELATIONS} AND c.relkind IN ('v', 'm') ORDER BY c.relname COLLATE "C""#
    );
    let rows = query(executor, &sql, params(schema, None)).await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let mut columns = columns(executor, schema, None).await?;
    let mut triggers = triggers(executor, schema, None).await?;
    rows.iter()
        .map(|row| {
            let name = row.text("name")?;
            Ok(View {
                schema: schema.to_string(),
                materialized: row.bool("materialized")?,
                comment: row.opt("comment")?,
                columns: columns.remove(&name).unwrap_or_default(),
                triggers: triggers.remove(&name).unwrap_or_default(),
                definition: row.opt("definition")?,
                name,
            })
        })
        .collect()
}

pub async fn enums<E: Executor>(executor: &E, schema: &str) -> Result<Vec<Enum>, E> {
    query(
        executor,
        r#"SELECT t.typname::text AS name,
                  ARRAY(SELECT e.enumlabel::text FROM pg_catalog.pg_enum e
                        WHERE e.enumtypid = t.oid ORDER BY e.enumsortorder) AS labels,
                  pg_catalog.obj_description(t.oid, 'pg_type') AS comment
           FROM pg_catalog.pg_type t
           JOIN pg_catalog.pg_namespace n ON n.oid = t.typnamespace AND n.nspname = $1
           WHERE t.typtype = 'e' AND NOT EXISTS (
             SELECT 1 FROM pg_catalog.pg_depend e
             WHERE e.classid = 'pg_catalog.pg_type'::regclass AND e.objid = t.oid
               AND e.deptype = 'e'
           )
           ORDER BY t.typname COLLATE "C""#,
        vec![schema.into()],
    )
    .await?
    .iter()
    .map(|row| {
        Ok(Enum {
            schema: schema.to_string(),
            name: row.text("name")?,
            values: row.texts("labels")?,
            comment: row.opt("comment")?,
        })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nulls() {
        assert!(is_null("NULL") && is_null("NULL::character varying(10)[]"));
        assert!(!is_null("NULL::text || 'x'::text") && !is_null("NULLIF(1, 2)"));
    }

    #[test]
    fn types() {
        for (name, ty) in [
            ("character varying(10)", Some(Type::Text)),
            ("timestamp(3) without time zone", Some(Type::DateTime)),
            ("integer[]", Some(Type::Array(Box::new(Type::Int)))),
            (
                "public.geometry(PointZ,4326)",
                Some(Type::Geometry(GeoType::Point)),
            ),
            ("geometry", None),
            ("interval", None),
        ] {
            assert_eq!(column_type(name), ty, "{name}");
        }
    }
}
