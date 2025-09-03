use alloc::borrow::Cow;

use crate::{Context, Error, expr::Expression};

pub trait Set<'key, 'val> {
    fn set<F, V>(&mut self, field: F, value: V) -> &mut Self
    where
        F: Into<Cow<'key, str>>,
        V: Expression<'val> + Send + Sync + Clone + 'val;

    fn with<F, V>(mut self, field: F, value: V) -> Self
    where
        Self: Sized,
        F: Into<Cow<'key, str>>,
        V: Expression<'val> + Send + Sync + Clone + 'val,
    {
        self.set(field, value);
        self
    }

    fn build(self, ctx: &mut Context<'val>) -> Result<(), Error>;
}
