use nonempty::NonEmpty;
use usql_core::{ColumnIndex, Connector};

use crate::{
    project::{Project, ProjectField, ProjectRelation, RelationKind},
    writer::{RowWriter, Unpack},
};

pub struct Row<'a, T: usql_core::Row> {
    rows: Vec<T>,
    project: &'a Project<'a>,
}

impl<'a, T: usql_core::Row> Unpack for Row<'a, T>
where
    <T::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
{
    fn unpack<W: crate::writer::RowWriter>(&self, mut writer: W) -> Result<W::Output, W::Error> {
        self.project.write(&mut writer, &self.rows);

        writer.finish()
    }
}

impl Project<'_> {
    fn write<W: RowWriter, T: usql_core::Row>(&self, writer: &mut W, rows: &[T])
    where
        <T::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
        W: RowWriter,
    {
        let Some(first) = rows.first() else {
            todo!("Empty")
        };

        for field in &self.fields {
            field.write(first, writer);
        }
    }
}

impl<'a> ProjectField<'a> {
    fn write<O, R>(&self, row: &R, writer: &mut O)
    where
        R: usql_core::Row,
        <R::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
        O: RowWriter,
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

        writer.set_value(key, value)?;
    }
}

impl ProjectRelation<'_> {
    fn write<W, T>(&self, writer: &mut W, rows: &[T])
    where
        W: RowWriter,
        T: usql_core::Row,
    {
        let iter = rows.iter().enumerate().peekable();

        match self.kind {
            RelationKind::Many => {}
            RelationKind::One => {
                let Some((idx, row)) = iter.next() else {
                    return Ok(());
                };

                let pk = row.get(self.pk)?;

                if pk.as_ref().is_null() {
                    return Ok(());
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

                writer.set_one(
                    &self.name,
                    RowRef {
                        pk: &self.pk,
                        relations: &self.relations,
                        fields: &self.fields,
                        rows: &rows[idx..end],
                    },
                );
            }
        }
    }
}

pub struct RowRef<'a, T: usql_core::Row> {
    pub(crate) pk: &'a ColumnIndex<'a>,
    pub(crate) relations: &'a Vec<ProjectRelation<'a>>,
    pub(crate) fields: &'a Vec<ProjectField<'a>>,
    pub(crate) rows: &'a [T],
}

impl<'a, T: usql_core::Row> Unpack for RowRef<'a, T> {
    fn unpack<W: RowWriter>(&self, writer: W) -> Result<W::Output, W::Error> {
        todo!()
    }
}
