use alloc::borrow::Cow;

pub struct CreateVirtualTable<'a> {
    name: Cow<'a, str>,
}
