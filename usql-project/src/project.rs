use std::sync::Arc;

use usql_value::{Atom, Type};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnIndex {
    Named(Atom),
    Index(usize),
}

impl<'a> From<&'a ColumnIndex> for usql_core::ColumnIndex<'a> {
    fn from(value: &'a ColumnIndex) -> Self {
        match value {
            ColumnIndex::Index(idx) => usql_core::ColumnIndex::Index(*idx),
            ColumnIndex::Named(named) => usql_core::ColumnIndex::Named(named),
        }
    }
}

impl From<Atom> for ColumnIndex {
    fn from(value: Atom) -> Self {
        ColumnIndex::Named(value)
    }
}

impl<'a> From<&'a str> for ColumnIndex {
    fn from(value: &'a str) -> Self {
        ColumnIndex::Named(value.into())
    }
}

impl From<i32> for ColumnIndex {
    fn from(value: i32) -> Self {
        ColumnIndex::Index(value as _)
    }
}

impl From<usize> for ColumnIndex {
    fn from(value: usize) -> Self {
        ColumnIndex::Index(value as _)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectInner {
    pub(crate) pk: ColumnIndex,
    pub(crate) fields: Vec<ProjectField>,
    pub(crate) relations: Vec<ProjectRelation>,
}

#[derive(Clone, Debug)]
pub struct Project(Arc<ProjectInner>);

impl Project {
    pub(crate) fn inner(&self) -> &ProjectInner {
        &self.0
    }
}

impl Project {
    pub fn new(pk: impl Into<ColumnIndex>) -> Project {
        Project(Arc::new(ProjectInner {
            pk: pk.into(),
            relations: Default::default(),
            fields: Default::default(),
        }))
    }

    pub fn field(mut self, map: ProjectField) -> Self {
        Arc::make_mut(&mut self.0).fields.push(map);
        self
    }

    pub fn add_field(&mut self, map: ProjectField) -> &mut Self {
        Arc::make_mut(&mut self.0).fields.push(map);
        self
    }

    pub fn relation(mut self, relation: ProjectRelation) -> Self {
        Arc::make_mut(&mut self.0).relations.push(relation);
        self
    }

    pub fn add_relation(&mut self, relation: ProjectRelation) -> &mut Self {
        Arc::make_mut(&mut self.0).relations.push(relation);
        self
    }
}

#[derive(Clone, Debug)]
pub struct ProjectField {
    pub(crate) index: ColumnIndex,
    pub(crate) map: Option<Atom>,
    pub(crate) ty: Option<Type>,
}

impl ProjectField {
    pub fn new(index: impl Into<ColumnIndex>) -> ProjectField {
        ProjectField {
            index: index.into(),
            map: None,
            ty: None,
        }
    }

    pub fn map(mut self, mapping: impl Into<Atom>) -> Self {
        self.map = Some(mapping.into());
        self
    }

    pub fn ty(mut self, ty: Type) -> Self {
        self.ty = Some(ty);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Many,
    One,
}

#[derive(Clone, Debug)]
pub struct ProjectRelation {
    pub(crate) kind: RelationKind,
    pub(crate) pk: ColumnIndex,
    pub(crate) name: Atom,
    pub(crate) relations: Vec<ProjectRelation>,
    pub(crate) fields: Vec<ProjectField>,
}

impl ProjectRelation {
    pub fn from_project(
        project: Project,
        kind: RelationKind,
        name: impl Into<Atom>,
    ) -> ProjectRelation {
        let project = Arc::try_unwrap(project.0).unwrap_or_else(|m| (*m).clone());

        ProjectRelation {
            kind,
            pk: project.pk,
            name: name.into(),
            relations: project.relations,
            fields: project.fields,
        }
    }

    pub fn single(index: impl Into<ColumnIndex>, name: impl Into<Atom>) -> ProjectRelation {
        ProjectRelation {
            kind: RelationKind::One,
            pk: index.into(),
            name: name.into(),
            relations: Default::default(),
            fields: Default::default(),
        }
    }

    pub fn many(index: impl Into<ColumnIndex>, name: impl Into<Atom>) -> ProjectRelation {
        ProjectRelation {
            kind: RelationKind::Many,
            pk: index.into(),
            name: name.into(),
            relations: Default::default(),
            fields: Default::default(),
        }
    }

    pub fn field(mut self, map: ProjectField) -> Self {
        self.fields.push(map);
        self
    }

    pub fn relation(mut self, relation: ProjectRelation) -> Self {
        self.relations.push(relation);
        self
    }
}
