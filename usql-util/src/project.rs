use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    iter::Peekable,
    pin::Pin,
};

use futures::{Stream, StreamExt, pin_mut};
use smallvec::SmallVec;
use usql_core::{ColumnIndex, Connector, Row};
use usql_value::{Type, Value, ValueCow};

pub struct Project<'a, const N: usize = 8> {
    count: usize,
    pk: usize,
    relations: SmallVec<[ProjectRelation<'a>; N]>,
    fields: SmallVec<[ProjectField<'a>; N]>,
}

impl<'a, const N: usize> Project<'a, N> {
    pub fn new(columns: usize, pk: usize) -> Project<'a, N> {
        let relations = SmallVec::with_capacity(columns);
        let fields = SmallVec::with_capacity(columns);

        Project {
            pk,
            count: columns,
            relations,
            fields,
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

#[derive(Clone)]
pub struct ProjectField<'a> {
    pub index: usize,
    pub map: Option<Cow<'a, str>>,
    pub ty: Option<Type>,
}

impl<'a> ProjectField<'a> {
    fn write<B, O>(&self, row: &B::Row, writer: &mut O::Writer) -> Result<(), B::Error>
    where
        B: Connector,
        O: Output,
    {
        let value = match &self.ty {
            Some(v) => row.get_typed(ColumnIndex::Index(self.index), v.clone())?,
            None => row.get(ColumnIndex::Index(self.index))?,
        };

        let key = self
            .map
            .as_ref()
            .map(|m| m.as_ref())
            .or_else(|| row.column_name(self.index))
            .expect("key");

        writer.write_field(key, value);

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Many,
    One,
}

#[derive(Clone)]
pub struct ProjectRelation<'a> {
    kind: RelationKind,
    index: usize,
    name: Cow<'a, str>,
    relations: Vec<ProjectRelation<'a>>,
    fields: Vec<ProjectField<'a>>,
}

impl<'a> ProjectRelation<'a> {
    pub fn single(index: usize, name: impl Into<Cow<'a, str>>) -> ProjectRelation<'a> {
        ProjectRelation {
            kind: RelationKind::One,
            index,
            name: name.into(),
            relations: Default::default(),
            fields: Default::default(),
        }
    }

    pub fn many(index: usize, name: impl Into<Cow<'a, str>>) -> ProjectRelation<'a> {
        ProjectRelation {
            kind: RelationKind::Many,
            index,
            name: name.into(),
            relations: Default::default(),
            fields: Default::default(),
        }
    }
}

impl<'a> ProjectRelation<'a> {
    fn write<B, O>(&self, row: &B::Row, output: &mut O::Writer) -> Result<(), B::Error>
    where
        B: Connector,
        O: Output,
    {
        let pk = row.get(ColumnIndex::Index(self.index))?;

        if pk.as_ref().is_null() {
            return Ok(());
        }

        let mut entry = O::create();

        for field in &self.fields {
            field.write::<B, O>(&row, &mut entry)?;
        }

        for relation in &self.relations {
            relation.write::<B, O>(&row, &mut entry)?;
        }

        match self.kind {
            RelationKind::Many => {
                output.append_relation(&self.name, entry.finalize());
            }
            RelationKind::One => {
                output.write_relation(&self.name, entry.finalize());
            }
        }

        Ok(())
    }
}

macro_rules! ok_or {
    ($expr: expr) => {
        match $expr {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err)),
        }
    };
}

impl<'a, const N: usize> Project<'a, N> {
    pub fn next_row<B, O, T>(
        &self,
        iter: &mut Peekable<T>,
    ) -> Option<Result<<O::Writer as Writer>::Output, B::Error>>
    where
        O: Output,
        B: Connector,
        T: Iterator<Item = Result<B::Row, B::Error>>,
    {
        let next = match iter.next()? {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err)),
        };

        let pk = match next.get(ColumnIndex::Index(self.pk)) {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err)),
        };

        let mut output = O::create();

        for field in &self.fields {
            ok_or!(field.write::<B, O>(&next, &mut output));
        }

        for relation in &self.relations {
            ok_or!(relation.write::<B, O>(&next, &mut output))
        }

        loop {
            if let Some(peek) = iter.peek() {
                let Ok(ret) = peek else {
                    break;
                };

                let next_pk = match ret.get(ColumnIndex::Index(self.pk)) {
                    Ok(ret) => ret,
                    Err(_) => break,
                };

                if next_pk != pk {
                    break;
                }
            }

            let Some(next) = iter.next() else {
                break;
            };

            let next = ok_or!(next);

            for relation in &self.relations {
                ok_or!(relation.write::<B, O>(&next, &mut output))
            }
        }

        Some(Ok(output.finalize()))
    }

    pub async fn next_row_async<B, O, T>(
        &self,
        iter: &mut futures::stream::Peekable<T>,
    ) -> Option<Result<<O::Writer as Writer>::Output, B::Error>>
    where
        O: Output,
        B: Connector,
        T: Stream<Item = Result<B::Row, B::Error>> + Unpin,
    {
        let mut iter = Pin::new(iter);

        let next = match iter.next().await? {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err)),
        };

        let pk = match next.get(ColumnIndex::Index(self.pk)) {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err)),
        };

        let mut output = O::create();

        for field in &self.fields {
            ok_or!(field.write::<B, O>(&next, &mut output));
        }

        for relation in &self.relations {
            ok_or!(relation.write::<B, O>(&next, &mut output))
        }

        loop {
            if let Some(peek) = iter.as_mut().peek().await {
                let Ok(ret) = peek else {
                    break;
                };

                let next_pk = match ret.get(ColumnIndex::Index(self.pk)) {
                    Ok(ret) => ret,
                    Err(_) => break,
                };

                if next_pk != pk {
                    break;
                }
            }

            let Some(next) = iter.next().await else {
                break;
            };

            let next = ok_or!(next);

            for relation in &self.relations {
                ok_or!(relation.write::<B, O>(&next, &mut output))
            }
        }

        Some(Ok(output.finalize()))
    }
}

pub trait Output {
    type Writer: Writer;

    fn create() -> Self::Writer;
}

pub trait Writer {
    type Output;
    fn write_field(&mut self, field: &str, value: ValueCow<'_>);
    fn write_relation(&mut self, field: &str, value: Self::Output);
    fn append_relation(&mut self, field: &str, value: Self::Output);
    fn finalize(self) -> Self::Output;
}
