#![allow(clippy::manual_async_fn)]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod any_connector;
#[cfg(feature = "std")]
pub mod config;
mod system;
mod traits;
pub mod util;
pub mod value;

pub use self::{any_connector::*, system::*, traits::*, value::*};

#[cfg(feature = "libsql")]
mod libsql_impl;
#[cfg(feature = "libsql")]
pub use libsql_impl::*;

#[cfg(feature = "sqlite")]
mod sqlite_impl;
#[cfg(feature = "sqlite")]
pub use sqlite_impl::*;

#[cfg(feature = "postgres")]
mod postgres_impl;
#[cfg(feature = "postgres")]
pub use postgres_impl::*;

pub mod prelude {
    pub use super::traits::*;
}

pub use chrono;
