//! Conversion to and from PostgreSQL's binary format.

mod decode;
mod encode;
mod numeric;

type BoxError = alloc::boxed::Box<dyn core::error::Error + Sync + Send>;

#[cfg(test)]
mod tests;
