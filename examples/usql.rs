use usql::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let sqlite = usql::AnyConnector::create_pool(
        usql::SqliteOptions {
            path: None,
            flags: Default::default(),
        }
        .into(),
    )
    .await
    .unwrap();

    let conn = sqlite.get().await.unwrap();

    let mut stmt = conn.prepare("CREATE TABLE test(name TEST)").await.unwrap();

    conn.exec(&mut stmt, vec![]).await.unwrap();
}
