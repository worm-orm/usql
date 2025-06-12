use usql_builder::{
    StatementExt,
    expr::*,
    mutate::{Set, insert, update},
    schema::{Column, ColumnType, CreateIndex, create_table},
    select::{
        FilterQuery, GroupQuery, IdentExt, Join, JoinQuery, LimitQuery, Order, QueryExt, Select,
        SortQuery, TargetExt, select,
    },
};
use usql_core::System;

fn main() {
    let sql = create_table("users")
        .column(Column::new("id", ColumnType::Int).auto(true).primary_key())
        .column(Column::new("name", ColumnType::VarChar(100)).required(true))
        .column(Column::new("status", ColumnType::VarChar(4)).default(val("OK")))
        .force()
        .to_sql(System::Sqlite)
        .unwrap();

    println!("{sql}");

    let sql = CreateIndex::new("users", "users_name_index", vec!["name".into()])
        .unique(true)
        .to_sql(System::Sqlite)
        .unwrap();

    println!("{sql}");

    let sql = insert("users")
        .with("name", "Rasmus")
        .returning("id")
        .to_sql(System::Sqlite)
        .unwrap();

    println!("{sql}");

    let sql = update("user")
        .with("status", val("FAIL"))
        .filter("name".eql(val("Rasmus")))
        .returning("id")
        .to_sql(System::Sqlite)
        .unwrap();

    println!("{sql}");

    let users = "users".alias("users");

    let user_name = users.col("name").alias("user__name");
    let user_id = users.col("id").alias("user__id");

    let test = call("max", (val(20), val(10)));

    let subselect = select("test", ("id", "age")).into_stmt();

    let select = select(
        users,
        (
            user_name,
            user_id,
            test.alias("max"),
            subselect.alias("sub"),
            switch(
                user_id,
                (when(val(20), val("Hello, Friend!")),),
                val("Stranger"),
            )
            .alias("status"),
            ifelse((when(user_id.lte(val(10)), val("Yo")),), val("No")).alias("yay_or_nay"),
        ),
    )
    .join(Join::left("blogs".alias("blogs")).on("blogs".alias("blogs").col("user_id").eql(user_id)))
    .filter(
        user_name
            .like(val("%rasmus%"))
            .or(user_id.eql(val(20)))
            .and(user_id.has(subselect)),
    )
    .group_by(user_name)
    .having(user_id.eql(val(20)))
    .order_by((
        (user_name, Order::Asc),
        (call("count", (user_id,)), Order::Desc),
    ))
    .limit(0, 100)
    .into_stmt()
    .to_sql(System::Sqlite)
    .unwrap();

    println!("{select}");
}
