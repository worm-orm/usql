use usql::Connector;

use crate::migration::{DynamicRunner, Runner};

pub struct LoadContext<'a, B: Connector> {
    migrations: &'a mut Vec<Box<dyn DynamicRunner<B>>>,
}

pub trait MigrationLoader<B> {
    type Migration: Runner<B, Loader = Self>;
    type Error;
}
