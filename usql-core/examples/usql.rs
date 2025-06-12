use usql_core::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let sqlite = usql_core::AnyConnector::create_pool(
        usql_core::SqliteOptions {
            path: None,
            flags: Default::default(),
        }
        .into(),
    )
    .await
    .unwrap();

    let conn = sqlite.get().await.unwrap();

    let mut stmt = conn.prepare("CREATE TABLE test(name TEXT)").await.unwrap();

    conn.exec(&mut stmt, vec![]).await.unwrap();
}
