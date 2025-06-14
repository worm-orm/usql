use futures::TryStreamExt;
use usql::{Error, FromRow};
use usql_builder::{
    expr::val,
    mutate::{Set, insert},
    schema::{Column, ColumnType, create_table},
    select::{QueryExt, select},
};
use usql_core::Connector;
use usql_sqlite::SqliteOptions;

#[derive(Debug, FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

fn main() {
    futures::executor::block_on(async move {
        let pool = usql_sqlite::Sqlite::create_pool(SqliteOptions::default())
            .await
            .unwrap();

        let core = usql::Pool::<usql_sqlite::Sqlite>::new(pool);

        let conn = core.conn().await.unwrap();

        conn.exec(
            create_table("user")
                .column(Column::new("id", ColumnType::Int).auto(true).primary_key())
                .column(Column::new("name", ColumnType::Text).required(true))
                .column(Column::new("email", ColumnType::Text).required(true)),
        )
        .await?;

        conn.exec(
            insert("user")
                .with("name", val("Rasmus"))
                .with("email", val("rasmus@email.com")),
        )
        .await?;

        let mut stream = conn
            .fetch(select("user", ("id", "name", "email")).into_stmt())
            .await?
            .into::<User>();

        while let Some(row) = stream.try_next().await? {
            println!("{:?}", row);
        }

        Result::<_, Error<usql_sqlite::Sqlite>>::Ok(())
    })
    .unwrap();
}
