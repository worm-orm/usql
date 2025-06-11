use crate::select::Query;
use core::fmt::Write;

pub struct LimitSelect<S> {
    select: S,
    offset: u64,
    limit: u64,
}

impl<S> LimitSelect<S> {
    pub fn new(select: S, offset: u64, limit: u64) -> LimitSelect<S> {
        LimitSelect {
            select,
            offset,
            limit,
        }
    }
}

impl<'a, S> Query<'a> for LimitSelect<S>
where
    S: Query<'a>,
{
    fn build(self, ctx: &mut crate::Context<'a>) -> Result<(), crate::Error> {
        self.select.build(ctx)?;

        write!(ctx, " LIMIT {} OFFSET {}", self.limit, self.offset)?;

        Ok(())
    }
}
