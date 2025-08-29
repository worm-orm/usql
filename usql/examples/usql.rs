use futures::TryStreamExt;
use usql::{Error, FromRow, IntoQuery};
use usql_builder::{
    StatementExt,
    expr::{ExpressionExt, val},
    mutate::{Set, insert},
    schema::{Column, ColumnType, ForeignKey, create_table},
    select::{IdentExt, Join, JoinQuery, Order, QueryExt, SortQuery, TargetExt, select},
};
use usql_core::{Connector, System};
use usql_sqlite::{Sqlite, SqliteOptions};
use usql_util::{DefaultOutput, Output, Project, ProjectField, ProjectRelation, Writer};

#[derive(Debug, FromRow)]
struct User {
    id: i32,
    name: String,
    email: String,
}

fn main() {
    // usql_sqlite::init_vector();

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
            create_table("blog")
                .column(Column::new("id", ColumnType::Int).auto(true).primary_key())
                .column(Column::new("title", ColumnType::Text).required(true))
                .column(
                    Column::new("user_id", ColumnType::Int)
                        .required(true)
                        .foreign_key(ForeignKey::new("user", "id")),
                ),
        )
        .await?;

        conn.exec(
            create_table("comment")
                .column(Column::new("id", ColumnType::Int).auto(true).primary_key())
                .column(Column::new("title", ColumnType::Text).required(true))
                .column(
                    Column::new("user_id", ColumnType::Int)
                        .required(true)
                        .foreign_key(ForeignKey::new("user", "id")),
                )
                .column(
                    Column::new("blog_id", ColumnType::Int)
                        .required(true)
                        .foreign_key(ForeignKey::new("blog", "id")),
                ),
        )
        .await?;

        conn.exec(
            insert("user")
                .with("name", val("Rasmus"))
                .with("email", val("rasmus@email.com")),
        )
        .await?;

        conn.exec(
            insert("user")
                .with("name", val("Wilbur"))
                .with("email", val("wilbur@email.com")),
        )
        .await?;

        for i in 0..2 {
            conn.exec(
                insert("blog")
                    .with("title", val(format!("Blog {i}")))
                    .with("user_id", val(1)),
            )
            .await?;

            for ii in 1..3 {
                conn.exec(
                    insert("comment")
                        .with("title", val(format!("Comment {i}-{ii}")))
                        .with("user_id", val(ii))
                        .with("blog_id", val(1)),
                )
                .await?;
            }
        }

        conn.exec(
            insert("blog")
                .with("title", val(format!("Blog")))
                .with("user_id", val(2)),
        )
        .await?;

        let mut stream = conn
            .fetch(select("user", ("id", "name", "email")).into_stmt())
            .await?
            .into::<User>();

        while let Some(row) = stream.try_next().await? {
            println!("{:?}", row);
        }

        let project = Project::new(6, "user_id")
            .field(ProjectField::new(0).map("id"))
            .field(ProjectField::new(1))
            .relation(
                ProjectRelation::many(3, "blogs")
                    .field(ProjectField::new("blog_id").map("id"))
                    .field(ProjectField::new(4).map("title"))
                    .relation(
                        ProjectRelation::many(5, "comments")
                            .field(ProjectField::new(6))
                            .relation(
                                ProjectRelation::single(7, "user").field(ProjectField::new(7)),
                            ),
                    ),
            );

        let stmt = select(
            "user",
            (
                "user".col("id").alias("user_id"),
                "user".col("name").alias("user_name"),
                "user".col("email").alias("user_email"),
                "blog".col("id").alias("blog_id"),
                "blog".col("title").alias("blog_title"),
                "comment".col("id").alias("comment_id"),
                "comment".col("title").alias("command_title"),
                "comment_user".col("id").alias("id"),
            ),
        )
        .join(Join::left("blog").on("blog".col("user_id").eql("user".col("id"))))
        .join(Join::left("comment").on("comment".col("blog_id").eql("blog".col("id"))))
        .join(
            Join::left("user".alias("comment_user"))
                .on("comment_user".col("id").eql("comment".col("user_id"))),
        )
        .order_by(("user_id", Order::Asc))
        .into_stmt();

        // println!("Stmt {}", stmt.to_sql(System::Sqlite).unwrap());

        let mut stream = conn.fetch(stmt).await?.project_into(project, DefaultOutput);

        // let mut stream = project.wrap_stream(SerdeOutput, stream.map_ok(|m| m.into_inner()));

        while let Some(row) = stream.try_next().await.unwrap() {
            println!("{}", serde_json::to_string_pretty(&row).unwrap());
        }

        let stmt = conn.fetch_one("SELECT vec_version()").await?;

        println!("Vector {}", stmt.try_get::<String, _>(0).unwrap());

        Result::<_, Error<usql_sqlite::Sqlite>>::Ok(())
    })
    .unwrap();
}
