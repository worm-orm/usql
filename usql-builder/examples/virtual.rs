use usql_builder::{
    Error, StatementExt,
    expr::{ExpressionExt, call},
    schema::create_virtual_table,
    select::{FilterQuery, IdentExt, QueryExt, select},
};
use usql_core::System;
use usql_value::Value;
use zerocopy::AsBytes;

fn main() -> Result<(), Error> {
    let sql = create_virtual_table("embed", "vec0")
        .arg("id INTEGER PRIMARY KEY")
        .arg("embeddings float[16]")
        .to_sql(System::LibSql)?;

    println!("{}", sql);

    let vec: Value = vec![0.2f32, 0.4, 0.4].into();

    let sql = select(
        "embed",
        (
            "id",
            "embeddings",
            call("vec_length", ("embeddings",)).alias("len"),
        ),
    )
    .filter("embeddings".matching(vec))
    .into_stmt()
    .to_sql(System::LibSql)?;

    println!("{}", sql);

    Ok(())
}
