use usql::{System, ValueRef};
use usql_builder::select::{
    FilterQuery, GroupQuery, IdentExt, Join, JoinQuery, LimitQuery, Order, Query, QueryExt,
    QueryStmt, Select, SortQuery, TargetExt, table,
};
use usql_builder::{Context, expr::*};

fn main() {
    let users = "users".alias("users");

    let user_name = users.col("name").alias("user__name");
    let user_id = users.col("id").alias("user__id");

    let test = call("max", (val(20), val(10)));

    let subselect = Select::new("test", ("id", "age")).into_stmt();

    let select = Select::new(
        users,
        (
            user_name,
            user_id,
            test.alias("max"),
            subselect.alias("sub"),
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
    .order_by((
        (user_name, Order::Asc),
        (call("count", (user_id,)), Order::Desc),
    ))
    .limit(0, 100);

    let mut ctx = Context::new(System::Mysql);

    select.build(&mut ctx).expect("build");

    let form = sqlformat::format(&ctx.to_string(), &Default::default(), &Default::default());

    println!("{form}");
}
