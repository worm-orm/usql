use core::fmt;

use deadpool_postgres::{Config, PoolConfig, Runtime};
use tokio_postgres::NoTls;

use crate::{error::Error, pool::Pool};

/// How connections are encrypted.
///
/// With the default `sslmode=prefer`, an unencrypted connection is used if the server
/// does not support TLS. Set `sslmode=require` to rule that out.
#[derive(Clone, Default)]
#[non_exhaustive]
pub enum Tls {
    #[default]
    NoTls,
    #[cfg(feature = "rustls")]
    Rustls(Box<rustls::ClientConfig>),
}

impl fmt::Debug for Tls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tls::NoTls => f.write_str("NoTls"),
            #[cfg(feature = "rustls")]
            Tls::Rustls(_) => f.write_str("Rustls"),
        }
    }
}

#[cfg(feature = "rustls")]
impl Tls {
    /// Trust the certificates trusted by the operating system
    pub fn native_roots() -> Result<Tls, Error> {
        let certs = rustls_native_certs::load_native_certs();
        let mut roots = rustls::RootCertStore::empty();
        roots.add_parsable_certificates(certs.certs);
        if roots.is_empty() {
            return Err(match certs.errors.into_iter().next() {
                Some(err) => Error::Tls(err.into()),
                None => Error::Tls("no trusted certificates found".into()),
            });
        }
        Tls::roots(roots)
    }

    /// Trust only the given certificates
    pub fn roots(roots: rustls::RootCertStore) -> Result<Tls, Error> {
        let provider = rustls::crypto::ring::default_provider();
        let config = rustls::ClientConfig::builder_with_provider(provider.into())
            .with_safe_default_protocol_versions()
            .map_err(|err| Error::Tls(err.into()))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Tls::Rustls(Box::new(config)))
    }
}

#[derive(Clone)]
pub struct Options {
    tls: Tls,
    config: Config,
}

impl Options {
    /// A connection string, either as an url or as key/value pairs
    pub fn from_url(url: impl Into<String>) -> Options {
        Options::from_config(Config {
            url: Some(url.into()),
            ..Config::default()
        })
    }

    pub fn from_config(config: Config) -> Options {
        Options {
            tls: Tls::default(),
            config,
        }
    }

    pub fn tls(mut self, tls: Tls) -> Self {
        self.tls = tls;
        self
    }

    /// The maximum number of connections in the pool
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.config
            .pool
            .get_or_insert_with(PoolConfig::default)
            .max_size = max_size;
        self
    }

    pub(crate) fn create_pool(self) -> Result<Pool, Error> {
        let runtime = Some(Runtime::Tokio1);
        let pool = match self.tls {
            Tls::NoTls => self.config.create_pool(runtime, NoTls)?,
            #[cfg(feature = "rustls")]
            Tls::Rustls(config) => self.config.create_pool(
                runtime,
                tokio_postgres_rustls::MakeRustlsConnect::new(*config),
            )?,
        };
        Ok(Pool(pool))
    }
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = f.debug_struct("Options");
        out.field("tls", &self.tls);
        // The url may contain a password, which is hidden by the parsed config
        match self.config.get_pg_config() {
            Ok(config) => out.field("config", &config),
            Err(_) => out.field("config", &"<invalid>"),
        };
        out.finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_password() {
        let options = Options::from_url("postgres://me:hunter2@localhost/db").max_size(3);
        let debug = format!("{options:?}");
        assert!(debug.contains("localhost"), "{debug}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert!(!format!("{:?}", Options::from_url("bad hunter2")).contains("hunter2"));
    }
}
