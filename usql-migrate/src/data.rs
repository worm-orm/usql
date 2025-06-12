use std::alloc::System;

use usql::{
    ColumnIndex, Connector, DatabaseInfo, Executor, JsonValue, Row, Type, chrono::NaiveDateTime,
    util::next,
};
use usql_builder::{
    StatementExt,
    expr::val,
    mutate::{Set, insert},
    schema::{Column, ColumnType, create_table},
    select::{Order, QueryExt, SortQuery, select},
};

use crate::{error::Error, migration::Migration};

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub date: NaiveDateTime,
    pub meta: JsonValue,
}

pub async fn ensure_table<E>(executor: &E, table: &str) -> Result<(), Error>
where
    E: Executor,
    <E::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
{
    let sql = create_table(table)
        .column(
            Column::new("name", ColumnType::Text)
                .required(true)
                .primary_key(),
        )
        .column(Column::new("data", ColumnType::DateTime).required(true))
        .column(Column::new("meta", ColumnType::Json))
        .to_sql(executor.db_info().variant())?;

    let mut stmt = executor.prepare(&sql.sql).await.unwrap();

    let params = sql
        .bindings
        .into_iter()
        .map(|m| m.to_owned())
        .collect::<Vec<_>>();

    executor.exec(&mut stmt, params).await.unwrap();

    Ok(())
}

pub async fn list_entries<E>(executor: &E, table: &str) -> Result<Vec<Entry>, Error>
where
    E: Executor,
    <E::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
{
    let sql = select(table, ("name", "date", "meta"))
        .order_by(("date", Order::Asc))
        .into_stmt()
        .to_sql(executor.db_info().variant())?;

    let mut stmt = executor.prepare(&sql.sql).await.unwrap();

    let params = sql
        .bindings
        .into_iter()
        .map(|m| m.to_owned())
        .collect::<Vec<_>>();

    let mut stream = executor.query(&mut stmt, params);

    let mut output = Vec::new();

    while let Some(row) = next(&mut stream).await {
        let row = row.unwrap();

        let name = row
            .get_typed(ColumnIndex::Index(0), Type::Text)
            .unwrap()
            .to_owned();
        let date = row
            .get_typed(ColumnIndex::Index(1), Type::DateTime)
            .unwrap()
            .to_owned();
        let meta = row
            .get_typed(ColumnIndex::Index(2), Type::Json)
            .unwrap()
            .to_owned();

        let entry = Entry {
            name: name.try_into().unwrap(),
            date: date.try_into().unwrap(),
            meta: meta.try_into().unwrap(),
        };

        output.push(entry);
    }

    Ok(output)
}

pub async fn insert_migration<E>(
    executor: &E,
    table: &str,
    name: &str,
    date: NaiveDateTime,
) -> Result<(), Error>
where
    E: Executor,
    <E::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
{
    let sql = insert(table)
        .with("name", val(name))
        .with("date", val(date))
        .to_sql(executor.db_info().variant())?;

    let mut stmt = executor.prepare(&sql.sql).await.unwrap();

    let params = sql
        .bindings
        .into_iter()
        .map(|m| m.to_owned())
        .collect::<Vec<_>>();

    executor.exec(&mut stmt, params).await.unwrap();

    Ok(())
}
