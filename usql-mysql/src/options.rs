use core::fmt;

use mysql_async::{Opts, OptsBuilder, PoolConstraints};

use crate::{error::Error, pool::Pool};

#[derive(Clone)]
enum Source {
    Url(String),
    Opts(Opts),
}

/// Connection options.
///
/// TLS is configured with the url parameters of mysql_async (`require_ssl=true`, ...),
/// or with [Options::from_opts], and needs the `rustls` feature.
#[derive(Clone)]
pub struct Options {
    source: Source,
    max_size: Option<usize>,
}

impl Options {
    /// A connection url, like `mysql://user:pass@localhost:3306/db`.
    /// It is parsed when the pool is created.
    pub fn from_url(url: impl Into<String>) -> Options {
        Options {
            source: Source::Url(url.into()),
            max_size: None,
        }
    }

    pub fn from_opts(opts: impl Into<Opts>) -> Options {
        Options {
            source: Source::Opts(opts.into()),
            max_size: None,
        }
    }

    /// The maximum number of connections in the pool
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.max_size = Some(max_size);
        self
    }

    fn opts(&self) -> Result<Opts, Error> {
        match &self.source {
            Source::Url(url) => Ok(Opts::from_url(url)?),
            Source::Opts(opts) => Ok(opts.clone()),
        }
    }

    pub(crate) fn create_pool(self) -> Result<Pool, Error> {
        let mut opts = self.opts()?;
        if let Some(max) = self.max_size {
            let max = max.max(1);
            let min = opts.pool_opts().constraints().min().min(max);
            let constraints = PoolConstraints::new(min, max).expect("min <= max");
            let pool_opts = opts.pool_opts().clone().with_constraints(constraints);
            opts = OptsBuilder::from_opts(opts).pool_opts(pool_opts).into();
        }
        Ok(Pool(mysql_async::Pool::new(opts)))
    }
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = f.debug_struct("Options");
        // Leave out the password
        match self.opts() {
            Ok(opts) => out
                .field("host", &opts.ip_or_hostname())
                .field("port", &opts.tcp_port())
                .field("user", &opts.user())
                .field("db_name", &opts.db_name()),
            Err(_) => out.field("opts", &"<invalid>"),
        };
        out.field("max_size", &self.max_size);
        out.finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_password() {
        let options = Options::from_url("mysql://me:hunter2@localhost/db").max_size(3);
        let debug = format!("{options:?}");
        assert!(debug.contains("localhost"), "{debug}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert!(!format!("{:?}", Options::from_url("bad hunter2")).contains("hunter2"));
    }
}
