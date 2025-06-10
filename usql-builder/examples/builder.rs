use usql::{System, ValueRef};
use usql_builder::select::{FilterQuery, Join, JoinQuery, Query, Select};
use usql_builder::{Context, expr::*};

fn main() {
    let select = Select::new("users", "rasmus")
        .join(Join::left("blogs").on("user_id".eql("id")))
        .filter("name".like(val("%rasmus%")).or("id".eql(val(20))));

    let mut ctx = Context::new(System::Sqlite);

    select.build(&mut ctx).expect("build");

    println!("{ctx}");
}
