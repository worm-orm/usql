use std::path::PathBuf;

use rquickjs::{Ctx, FromJs, JsLifetime, Object, class::Trace};
use rquickjs_util::{StringRef, throw, throw_if};
use usql::{
    AnyPool, Pool,
    config::{LibSqlConfig, SqliteConfig},
};

use crate::JsConn;

#[rquickjs::class(rename = "Pool")]
pub struct JsPool {
    pub inner: AnyPool,
}

impl<'js> Trace<'js> for JsPool {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

unsafe impl JsLifetime<'_> for JsPool {
    type Changed<'to> = JsPool;
}

#[rquickjs::methods]
impl JsPool {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'_>) -> rquickjs::Result<JsPool> {
        throw!(ctx, "Use Pool.open")
    }

    #[qjs(static)]
    async fn open(ctx: Ctx<'_>, config: Config) -> rquickjs::Result<JsPool> {
        let pool = throw_if!(ctx, config.0.crate_pool().await);
        Ok(JsPool { inner: pool })
    }

    async fn get(&self, ctx: Ctx<'_>) -> rquickjs::Result<JsConn> {
        let conn = throw_if!(ctx, self.inner.get().await);
        Ok(JsConn { conn })
    }
}

pub struct Config(pub usql::config::Config);

impl<'js> FromJs<'js> for Config {
    fn from_js(ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        let obj = Object::from_js(ctx, value)?;

        let ty = obj.get::<_, StringRef>("type")?;

        let cfg = match ty.as_str() {
            "sqlite" => {
                let path: Option<String> = obj.get("path")?;
                usql::config::Config {
                    workers: None,
                    kind: usql::config::DatabaseConfig::Sqlite(
                        path.map(|m| SqliteConfig::Path(PathBuf::from(m)))
                            .unwrap_or(SqliteConfig::Memory),
                    ),
                }
            }
            "libsql" => {
                let path: Option<String> = obj.get("path")?;
                usql::config::Config {
                    workers: None,
                    kind: usql::config::DatabaseConfig::LibSql(
                        path.map(|m| LibSqlConfig::Path(PathBuf::from(m)))
                            .unwrap_or(LibSqlConfig::Memory),
                    ),
                }
            }
            _ => {
                return Err(rquickjs::Error::new_from_js_message(
                    "string",
                    "database type",
                    "Sqlite, libsql or postgres",
                ));
            }
        };

        Ok(Config(cfg))
    }
}
