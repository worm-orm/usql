use std::alloc::System;

use usql::{Connector, DatabaseInfo, Executor, JsonValue, chrono::NaiveDateTime, util::next};
use usql_builder::{
    StatementExt,
    schema::{Column, ColumnType, create_table},
    select::{Order, QueryExt, SortQuery, select},
};

use crate::error::Error;

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

    while let Some(row) = next(&mut stream).await {}

    todo!()
}
