use std::borrow::Cow;

use usql_core::ColumnIndex;
use usql_value::Type;

pub struct Project<'a> {
    pub(crate) pk: ColumnIndex<'a>,
    pub(crate) fields: Vec<ProjectField<'a>>,
    pub(crate) relations: Vec<ProjectRelation<'a>>,
}

impl<'a> Project<'a> {
    pub fn new(pk: impl Into<ColumnIndex<'a>>) -> Project<'a> {
        Project {
            pk: pk.into(),
            relations: Default::default(),
            fields: Default::default(),
        }
    }

    pub fn field(mut self, map: ProjectField<'a>) -> Self {
        self.fields.push(map);
        self
    }

    pub fn add_field(&mut self, map: ProjectField<'a>) -> &mut Self {
        self.fields.push(map);
        self
    }

    pub fn relation(mut self, relation: ProjectRelation<'a>) -> Self {
        self.relations.push(relation);
        self
    }

    pub fn add_relation(&mut self, relation: ProjectRelation<'a>) -> &mut Self {
        self.relations.push(relation);
        self
    }
}

#[derive(Clone, Debug)]
pub struct ProjectField<'a> {
    pub(crate) index: ColumnIndex<'a>,
    pub(crate) map: Option<Cow<'a, str>>,
    pub(crate) ty: Option<Type>,
}

impl<'a> ProjectField<'a> {
    pub fn new(index: impl Into<ColumnIndex<'a>>) -> ProjectField<'a> {
        ProjectField {
            index: index.into(),
            map: None,
            ty: None,
        }
    }

    pub fn map(mut self, mapping: impl Into<Cow<'a, str>>) -> Self {
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
pub struct ProjectRelation<'a> {
    pub(crate) kind: RelationKind,
    pub(crate) pk: ColumnIndex<'a>,
    pub(crate) name: Cow<'a, str>,
    pub(crate) relations: Vec<ProjectRelation<'a>>,
    pub(crate) fields: Vec<ProjectField<'a>>,
}

impl<'a> ProjectRelation<'a> {
    pub fn from_project(
        project: Project<'a>,
        kind: RelationKind,
        name: impl Into<Cow<'a, str>>,
    ) -> ProjectRelation<'a> {
        ProjectRelation {
            kind,
            pk: project.pk,
            name: name.into(),
            relations: project.relations,
            fields: project.fields,
        }
    }

    pub fn single(
        index: impl Into<ColumnIndex<'a>>,
        name: impl Into<Cow<'a, str>>,
    ) -> ProjectRelation<'a> {
        ProjectRelation {
            kind: RelationKind::One,
            pk: index.into(),
            name: name.into(),
            relations: Default::default(),
            fields: Default::default(),
        }
    }

    pub fn many(
        index: impl Into<ColumnIndex<'a>>,
        name: impl Into<Cow<'a, str>>,
    ) -> ProjectRelation<'a> {
        ProjectRelation {
            kind: RelationKind::Many,
            pk: index.into(),
            name: name.into(),
            relations: Default::default(),
            fields: Default::default(),
        }
    }

    pub fn field(mut self, map: ProjectField<'a>) -> Self {
        self.fields.push(map);
        self
    }

    pub fn relation(mut self, relation: ProjectRelation<'a>) -> Self {
        self.relations.push(relation);
        self
    }
}
