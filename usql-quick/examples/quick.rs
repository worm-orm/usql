use rquickjs::{CatchResultExt, Module};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let vm = klaver::Options::default()
        .module::<usql_quick::Module>()
        .search_path(".")
        .build()
        .await
        .unwrap();

    klaver::async_with!(vm => |ctx| {

        let ret = Module::import(&ctx, "./usql-quick/examples/test.js").catch(&ctx)?;

        ret.into_future::<()>().await.catch(&ctx)?;

        Ok(())
    })
    .await
    .unwrap();
}
