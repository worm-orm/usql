use std::{borrow::Cow, collections::HashMap};

pub struct Project<'a> {
    pk: Cow<'a, str>,
    relations: HashMap<Cow<'a, str>, ProjectRelation<'a>>,
    fields: HashMap<Cow<'a, str>, Cow<'a, str>>,
}

pub enum RelationKind {
    Many,
    One,
}

pub struct ProjectRelation<'a> {
    kind: RelationKind,
    name: Cow<'a, str>,
    relations: HashMap<Cow<'a, str>, ProjectRelation<'a>>,
}
