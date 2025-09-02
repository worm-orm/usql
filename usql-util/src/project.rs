use std::{borrow::Cow, collections::HashMap, convert::Infallible, pin::Pin};

use anyhow::Context;
use futures::{
    Stream, StreamExt,
    stream::{BoxStream, LocalBoxStream},
};
use usql_core::{ColumnIndex, Connector, Row};
use usql_value::{Type, Value, ValueCow};

use crate::result::IntoResult;

pub struct Project<'a> {
    pk: ColumnIndex<'a>,
    relations: Vec<ProjectRelation<'a>>,
    fields: Vec<ProjectField<'a>>,
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

    pub fn relation(mut self, relation: ProjectRelation<'a>) -> Self {
        self.relations.push(relation);
        self
    }

    pub fn wrap_stream<'b, O, S>(
        self,
        output: O,
        stream: S,
    ) -> BoxStream<'b, anyhow::Result<<O::Writer as Writer>::Output>>
    where
        O: Output + Send + Sync + 'b,
        O::Writer: Send,
        <O::Writer as Writer>::Output: Send,
        <O::Writer as Writer>::Error: core::error::Error + Send + Sync + 'static,
        S: Stream + Send + Unpin + 'b,
        S::Item: IntoResult + Send,
        <S::Item as IntoResult>::Ok: Row,
        <S::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
        <<<S::Item as IntoResult>::Ok as Row>::Connector as Connector>::Error:
            core::error::Error + Send + Sync + 'static,
        'a: 'b,
    {
        let mut stream = stream.peekable();

        let stream = async_stream::stream! {
            let mut cache = Vec::new();


            while let Some(next) = self.next_row_async(&output,&mut stream, &mut cache).await {
                cache.clear();
                yield next;
            }

        };

        Box::pin(stream)
    }

    pub fn wrap_stream_local<'b, O, S>(
        self,
        output: O,
        stream: S,
    ) -> LocalBoxStream<'b, anyhow::Result<<O::Writer as Writer>::Output>>
    where
        O: Output + 'b,
        <O::Writer as Writer>::Error: core::error::Error + Send + Sync + 'static,
        S: Stream + Unpin + 'b,
        S::Item: IntoResult,
        <S::Item as IntoResult>::Ok: Row,
        <S::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
        <<<S::Item as IntoResult>::Ok as Row>::Connector as Connector>::Error:
            core::error::Error + Send + Sync + 'static,
        'a: 'b,
    {
        let mut stream = stream.peekable();

        let stream = async_stream::stream! {
            let mut cache = Vec::new();

            while let Some(next) = self.next_row_async(&output,&mut stream, &mut cache).await {
                cache.clear();
                yield next;
            }

        };

        Box::pin(stream)
    }
}

#[derive(Clone)]
pub struct ProjectField<'a> {
    index: ColumnIndex<'a>,
    map: Option<Cow<'a, str>>,
    ty: Option<Type>,
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

impl<'a> ProjectField<'a> {
    fn write<O, R>(&self, row: &R, writer: &mut O::Writer) -> anyhow::Result<()>
    where
        R: Row,
        <R::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
        O: Output,
        <O::Writer as Writer>::Error: core::error::Error + Send + Sync + 'static,
    {
        let value = match &self.ty {
            Some(v) => row.get_typed(self.index.clone(), v.clone())?,
            None => row.get(self.index.clone()).context("Get row")?,
        };

        let key = self
            .map
            .as_ref()
            .map(|m| m.as_ref())
            .or_else(|| match &self.index {
                ColumnIndex::Named(n) => Some(&*n),
                ColumnIndex::Index(idx) => row.column_name(*idx),
            })
            .expect("key");

        writer.write_field(key, value)?;

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
    pk: ColumnIndex<'a>,
    name: Cow<'a, str>,
    relations: Vec<ProjectRelation<'a>>,
    fields: Vec<ProjectField<'a>>,
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

impl<'a> ProjectRelation<'a> {
    fn write<O, R>(&self, output: &O, rows: &[R], result: &mut O::Writer) -> anyhow::Result<()>
    where
        R: Row,
        <R::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
        O: Output,
        <O::Writer as Writer>::Error: core::error::Error + Send + Sync + 'static,
    {
        let mut iter = rows.iter().enumerate().peekable();

        match self.kind {
            RelationKind::Many => {
                //
                let mut entries = Vec::<<O::Writer as Writer>::Output>::default();

                loop {
                    let Some((idx, row)) = iter.next() else { break };

                    let pk = row.get(self.pk)?;

                    if pk.as_ref().is_null() {
                        break;
                    }

                    let mut entry = output.create();

                    for field in &self.fields {
                        field.write::<O, _>(row, &mut entry)?;
                    }

                    let mut end = idx + 1;
                    loop {
                        let Some((_, next)) = iter.peek() else {
                            break;
                        };

                        let next_pk = next.get(self.pk)?;

                        if next_pk.as_ref().is_null() || next_pk != pk {
                            break;
                        };

                        let _ = iter.next();

                        end += 1;
                    }

                    for relation in &self.relations {
                        relation.write(output, &rows[idx..end], &mut entry)?;
                    }

                    entries.push(entry.finalize()?);
                }

                result.write_relations(&self.name, entries)?;
            }
            RelationKind::One => {
                let Some((idx, row)) = iter.next() else {
                    return Ok(());
                };

                let pk = row.get(self.pk)?;

                if pk.as_ref().is_null() {
                    return Ok(());
                }

                let mut entry = output.create();

                for field in &self.fields {
                    field.write::<O, _>(row, &mut entry)?;
                }

                let mut end = idx + 1;
                loop {
                    let Some((_, next)) = iter.peek() else {
                        break;
                    };

                    let next_pk = next.get(self.pk)?;

                    if next_pk.as_ref().is_null() || next_pk != pk {
                        break;
                    };

                    let _ = iter.next();

                    end += 1;
                }

                for relation in &self.relations {
                    relation.write(output, &rows[idx..end], &mut entry)?;
                }

                result.write_relation(&self.name, entry.finalize()?)?;
            }
        }

        Ok(())
    }
}

macro_rules! ok_or {
    ($expr: expr) => {
        match $expr {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err.into())),
        }
    };
}

impl<'a> Project<'a> {
    // pub fn next_row<B, O, T>(
    //     &self,
    //     iter: &mut Peekable<T>,
    // ) -> Option<anyhow::Result<<O::Writer as Writer>::Output>>
    // where
    //     O: Output,
    //     B: Connector,
    //     T: Iterator<Item = Result<B::Row, B::Error>>,
    // {
    //     let next = match iter.next()? {
    //         Ok(ret) => ret,
    //         Err(err) => return Some(Err(err)),
    //     };

    //     let pk = match next.get(ColumnIndex::Index(self.pk)) {
    //         Ok(ret) => ret,
    //         Err(err) => return Some(Err(err)),
    //     };

    //     let mut output = O::create();

    //     for field in &self.fields {
    //         ok_or!(field.write::<B, O>(&next, &mut output));
    //     }

    //     for relation in &self.relations {
    //         ok_or!(relation.write::<B, O>(&next, &mut output))
    //     }

    //     loop {
    //         if let Some(peek) = iter.peek() {
    //             let Ok(ret) = peek else {
    //                 break;
    //             };

    //             let next_pk = match ret.get(ColumnIndex::Index(self.pk)) {
    //                 Ok(ret) => ret,
    //                 Err(_) => break,
    //             };

    //             if next_pk != pk {
    //                 break;
    //             }
    //         }

    //         let Some(next) = iter.next() else {
    //             break;
    //         };

    //         let next = ok_or!(next);

    //         for relation in &self.relations {
    //             ok_or!(relation.write::<B, O>(&next, &mut output))
    //         }
    //     }

    //     Some(Ok(output.finalize()?))
    // }

    pub async fn next_row_async<O, T>(
        &self,
        output: &O,
        iter: &mut futures::stream::Peekable<T>,
        cache: &mut Vec<<T::Item as IntoResult>::Ok>,
    ) -> Option<anyhow::Result<<O::Writer as Writer>::Output>>
    where
        O: Output,
        <O::Writer as Writer>::Error: core::error::Error + Send + Sync + 'static,
        T: Stream + Unpin,
        T::Item: IntoResult,
        <T::Item as IntoResult>::Ok: Row,
        <T::Item as IntoResult>::Error: core::error::Error + Send + Sync + 'static,
        <<<T::Item as IntoResult>::Ok as Row>::Connector as Connector>::Error:
            core::error::Error + Send + Sync + 'static,
    {
        let mut iter = Pin::new(iter);

        let next = match iter.next().await?.into_result() {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err.into())),
        };

        let pk = match next.get(self.pk) {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err.into())),
        };

        let mut result = output.create();

        for field in &self.fields {
            ok_or!(field.write::<O, _>(&next, &mut result));
        }

        let pk = pk.to_owned();

        cache.push(next);

        loop {
            if let Some(peek) = iter.as_mut().peek().await {
                let Ok(ret) = peek.as_result() else {
                    break;
                };

                let next_pk = match ret.get(self.pk) {
                    Ok(ret) => ret,
                    Err(_) => break,
                };

                if next_pk.as_ref() != pk.as_ref() {
                    break;
                }
            }

            let Some(next) = iter.next().await else {
                break;
            };

            let next = ok_or!(next.into_result());

            cache.push(next);
        }

        for relation in &self.relations {
            ok_or!(relation.write(output, &cache, &mut result))
        }

        Some(result.finalize().map_err(Into::into))
    }
}

pub trait Output {
    type Writer: Writer;

    fn create(&self) -> Self::Writer;
}

pub trait Writer {
    type Output;
    type Error;
    fn write_field(&mut self, field: &str, value: ValueCow<'_>) -> Result<(), Self::Error>;
    fn write_relation(&mut self, field: &str, value: Self::Output) -> Result<(), Self::Error>;
    fn write_relations(&mut self, field: &str, value: Vec<Self::Output>)
    -> Result<(), Self::Error>;
    fn finalize(self) -> Result<Self::Output, Self::Error>;
}

#[derive(Default)]
pub struct DefaultOutput;

impl Output for DefaultOutput {
    type Writer = DefaultWriter;

    fn create(&self) -> Self::Writer {
        DefaultWriter {
            i: Default::default(),
        }
    }
}

pub struct DefaultWriter {
    i: HashMap<String, DefaultValue>,
}

impl Writer for DefaultWriter {
    type Error = Infallible;
    type Output = DefaultValue;

    fn write_field(&mut self, field: &str, value: ValueCow<'_>) -> Result<(), Self::Error> {
        self.i
            .insert(field.to_string(), DefaultValue::Scalar(value.to_owned()));
        Ok(())
    }

    fn write_relation(&mut self, field: &str, value: Self::Output) -> Result<(), Self::Error> {
        self.i.insert(field.to_string(), value);
        Ok(())
    }

    fn write_relations(
        &mut self,
        field: &str,
        value: Vec<Self::Output>,
    ) -> Result<(), Self::Error> {
        self.i.insert(field.to_string(), DefaultValue::List(value));
        Ok(())
    }

    fn finalize(self) -> Result<Self::Output, Self::Error> {
        Ok(DefaultValue::Map(self.i))
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DefaultValue {
    Map(HashMap<String, DefaultValue>),
    List(Vec<DefaultValue>),
    Scalar(Value),
}
