#![allow(clippy::manual_async_fn)]
#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// mod any_connector;
// #[cfg(feature = "std")]
// pub mod config;
mod system;
mod traits;
pub mod util;
// pub mod value;

pub use usql_value::*;

pub use self::{system::*, traits::*};

pub mod prelude {
    pub use super::traits::*;
}

// pub use chrono;
