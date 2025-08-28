use std::{borrow::Cow, iter::Peekable, pin::Pin};

use anyhow::{Context, anyhow};
use futures::{
    Stream, StreamExt, TryStream, TryStreamExt,
    stream::{BoxStream, LocalBoxStream},
};
use usql_core::{ColumnIndex, Connector, Row};
use usql_value::{Type, ValueCow};

use crate::result::IntoResult;

pub struct Project<'a> {
    count: usize,
    pk: usize,
    relations: Vec<ProjectRelation<'a>>,
    fields: Vec<ProjectField<'a>>,
}

impl<'a> Project<'a> {
    pub fn new(columns: usize, pk: usize) -> Project<'a> {
        Project {
            pk,
            count: columns,
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

            while let Some(next) = self.next_row_async(&output,&mut stream).await {
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

            while let Some(next) = self.next_row_async(&output,&mut stream).await {
                yield next;
            }

        };

        Box::pin(stream)
    }
}

#[derive(Clone)]
pub struct ProjectField<'a> {
    index: usize,
    map: Option<Cow<'a, str>>,
    ty: Option<Type>,
}

impl<'a> ProjectField<'a> {
    pub fn new(index: usize) -> ProjectField<'a> {
        ProjectField {
            index,
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
            Some(v) => row
                .get_typed(ColumnIndex::Index(self.index), v.clone())
                .context("Get row")?,
            None => row.get(ColumnIndex::Index(self.index)).context("Get row")?,
        };

        let key = self
            .map
            .as_ref()
            .map(|m| m.as_ref())
            .or_else(|| row.column_name(self.index))
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

        loop {
            let Some((idx, row)) = iter.next() else { break };

            let pk = row.get(ColumnIndex::Index(self.index))?;

            if pk.as_ref().is_null() {
                break;
            }

            match self.kind {
                RelationKind::Many => {
                    let mut entry = output.create();

                    for field in &self.fields {
                        field.write::<O, _>(row, &mut entry)?;
                    }

                    let mut end = idx + 1;
                    loop {
                        let Some((_, next)) = iter.peek() else {
                            break;
                        };

                        let next_pk = next.get(ColumnIndex::Index(self.index))?;

                        if next_pk.as_ref().is_null() || next_pk != pk {
                            break;
                        };

                        let _ = iter.next();

                        end += 1;
                    }

                    for relation in &self.relations {
                        relation.write::<O, _>(output, &rows[idx..end], &mut entry)?;
                    }
                    result.append_relation(&self.name, entry.finalize()?)?;
                }
                RelationKind::One => {
                    let mut entry = output.create();

                    for field in &self.fields {
                        field.write::<O, _>(row, &mut entry)?;
                    }

                    let mut end = idx + 1;
                    loop {
                        let Some((_, next)) = iter.peek() else {
                            break;
                        };

                        let next_pk = next.get(ColumnIndex::Index(self.index))?;

                        if next_pk.as_ref().is_null() || next_pk != pk {
                            break;
                        };

                        let _ = iter.next();

                        end += 1;
                    }

                    for relation in &self.relations {
                        relation.write::<O, _>(output, &rows[idx..end], &mut entry)?;
                    }

                    result.write_relation(&self.name, entry.finalize()?)?;

                    break;
                }
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

        let pk = match next.get(ColumnIndex::Index(self.pk)) {
            Ok(ret) => ret,
            Err(err) => return Some(Err(err.into())),
        };

        let mut result = output.create();

        for field in &self.fields {
            ok_or!(field.write::<O, _>(&next, &mut result));
        }

        let pk = pk.to_owned();

        let mut rest = vec![next];

        loop {
            if let Some(peek) = iter.as_mut().peek().await {
                let Ok(ret) = peek.as_result() else {
                    break;
                };

                let next_pk = match ret.get(ColumnIndex::Index(self.pk)) {
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

            rest.push(next);
        }

        for relation in &self.relations {
            ok_or!(relation.write(output, &rest, &mut result))
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
    fn append_relation(&mut self, field: &str, value: Self::Output) -> Result<(), Self::Error>;
    fn finalize(self) -> Result<Self::Output, Self::Error>;
}
