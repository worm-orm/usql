use crate::{
    ColumnIndex, Error, UnpackError,
    project::{Project, ProjectField, ProjectRelation, RelationKind},
    writer::{RowWriter, Unpack},
};
use usql_core::Connector;

pub struct Row<T: usql_core::Row> {
    pub(crate) rows: Vec<T>,
    pub(crate) project: Project,
}

impl<T: usql_core::Row> Row<T> {
    pub fn get(
        &self,
        idx: impl Into<ColumnIndex>,
    ) -> Result<usql_value::ValueCow<'_>, Error<T::Connector>> {
        let Some(row) = self.rows.first() else {
            return Err(Error::Unpack(UnpackError::new("Rows")));
        };

        let idx = idx.into();

        row.get((&idx).into()).map_err(Error::Connector)
    }

    pub fn get_typed(
        &self,
        idx: impl Into<ColumnIndex>,
        ty: usql_value::Type,
    ) -> Result<usql_value::ValueCow<'_>, Error<T::Connector>> {
        let Some(row) = self.rows.first() else {
            return Err(Error::Unpack(UnpackError::new("Rows")));
        };

        let idx = idx.into();
        row.get_typed((&idx).into(), ty).map_err(Error::Connector)
    }

    pub fn len(&self) -> usize {
        let Some(row) = self.rows.first() else {
            return 0;
        };

        row.len()
    }

    pub fn pk(&self) -> Result<usql_value::ValueCow<'_>, Error<T::Connector>> {
        let Some(row) = self.rows.first() else {
            return Err(Error::Unpack(UnpackError::new("Rows")));
        };

        let pk = &self.project.inner().pk;
        row.get(pk.into()).map_err(Error::Connector)
    }
}

impl<T: usql_core::Row> Unpack for Row<T>
where
    <T::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
{
    fn unpack<W: RowWriter>(&self, mut writer: W) -> Result<W::Output, UnpackError> {
        self.project.write(&mut writer, &self.rows)?;
        writer.finish()
    }
}

impl Project {
    fn write<W: RowWriter, T: usql_core::Row>(
        &self,
        writer: &mut W,
        rows: &[T],
    ) -> Result<(), UnpackError>
    where
        <T::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
        W: RowWriter,
    {
        let Some(first) = rows.first() else {
            return Err(UnpackError::new("Tried to unpack an empty row"));
        };

        for field in &self.inner().fields {
            field.write(first, writer)?;
        }

        for relation in &self.inner().relations {
            relation.write(writer, rows)?;
        }

        Ok(())
    }
}

impl ProjectField {
    fn write<O, R>(&self, row: &R, writer: &mut O) -> Result<(), UnpackError>
    where
        R: usql_core::Row,
        <R::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
        O: RowWriter,
    {
        let value = match &self.ty {
            Some(v) => row
                .get_typed((&self.index).into(), v.clone())
                .map_err(UnpackError::new)?,
            None => row.get((&self.index).into()).map_err(UnpackError::new)?,
        };

        let key = self
            .map
            .as_ref()
            .map(|m| m.as_ref())
            .or_else(|| match &self.index {
                ColumnIndex::Named(n) => Some(&*n),
                ColumnIndex::Index(idx) => row.column_name(*idx),
            })
            // Should never happen as we checked the validity of the index above
            .expect("key");

        writer.set_value(key, value)?;

        Ok(())
    }
}

impl ProjectRelation {
    fn write<W, T>(&self, writer: &mut W, rows: &[T]) -> Result<(), UnpackError>
    where
        W: RowWriter,
        T: usql_core::Row,
        <T::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
    {
        let mut iter = rows.iter().enumerate().peekable();

        match self.kind {
            RelationKind::Many => {
                let mut cache: Vec<RowRef<'_, T>> = Vec::with_capacity(1);

                loop {
                    let Some((idx, row)) = iter.next() else { break };

                    let pk = row.get((&self.pk).into()).map_err(UnpackError::new)?;

                    if pk.as_ref().is_null() {
                        break;
                    }

                    let mut end = idx + 1;
                    loop {
                        let Some((_, next)) = iter.peek() else {
                            break;
                        };

                        let next_pk = next.get((&self.pk).into()).map_err(UnpackError::new)?;

                        if next_pk.as_ref().is_null() || next_pk != pk {
                            break;
                        };

                        let _ = iter.next();

                        end += 1;
                    }

                    cache.push(RowRef {
                        fields: &self.fields,
                        relations: &self.relations,
                        rows: &rows[idx..end],
                    });
                }

                if !cache.is_empty() {
                    writer
                        .set_many(&self.name, cache.iter())
                        .map_err(UnpackError::new)?;
                }
            }
            RelationKind::One => {
                let Some((idx, row)) = iter.next() else {
                    return Ok(());
                };

                let pk = row.get((&self.pk).into()).map_err(UnpackError::new)?;

                if pk.as_ref().is_null() {
                    return Ok(());
                }

                let mut end = idx + 1;
                loop {
                    let Some((_, next)) = iter.peek() else {
                        break;
                    };

                    let next_pk = next.get((&self.pk).into()).map_err(UnpackError::new)?;

                    if next_pk.as_ref().is_null() || next_pk != pk {
                        break;
                    };

                    let _ = iter.next();

                    end += 1;
                }

                writer
                    .set_one(
                        &self.name,
                        RowRef {
                            relations: &self.relations,
                            fields: &self.fields,
                            rows: &rows[idx..end],
                        },
                    )
                    .map_err(UnpackError::new)?;
            }
        }

        Ok(())
    }
}

struct RowRef<'a, T: usql_core::Row> {
    pub relations: &'a Vec<ProjectRelation>,
    pub fields: &'a Vec<ProjectField>,
    pub rows: &'a [T],
}

impl<'a, T: usql_core::Row> Unpack for RowRef<'a, T>
where
    <T::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
{
    fn unpack<W: RowWriter>(&self, mut writer: W) -> Result<W::Output, UnpackError> {
        let Some(first) = self.rows.first() else {
            return Err(UnpackError::new("Tried to unpack an empty row"));
        };

        for field in self.fields {
            field.write(first, &mut writer)?;
        }

        for relation in self.relations {
            relation.write(&mut writer, self.rows)?;
        }

        writer.finish().map_err(UnpackError::new)
    }
}

#[cfg(feature = "serde")]
mod serialize {

    use serde::ser::{Error as _, Serialize, SerializeMap, Serializer};
    use usql_core::Connector;
    use usql_value::ValueCow;

    use crate::{RowWriter, Unpack, UnpackError};

    use super::Row;

    struct RowSerializer<S: SerializeMap> {
        s: S,
    }

    struct RowRel<R: Unpack> {
        row: R,
    }

    impl<S: SerializeMap> RowWriter for RowSerializer<S> {
        type Output = S::Ok;
        fn set_value(&mut self, field: &str, value: ValueCow<'_>) -> Result<(), UnpackError> {
            self.s
                .serialize_entry(field, &value.to_owned())
                .map_err(|err| UnpackError::new(err.to_string()))
        }
        fn set_many<I: Iterator>(&mut self, relation: &str, many: I) -> Result<(), UnpackError>
        where
            I::Item: Unpack,
        {
            self.s
                .serialize_entry(
                    relation,
                    &many.map(|row| RowRel { row }).collect::<Vec<_>>(),
                )
                .map_err(|err| UnpackError::new(err.to_string()))?;
            Ok(())
        }
        fn set_one<O: Unpack>(&mut self, relation: &str, row: O) -> Result<(), UnpackError> {
            self.s
                .serialize_entry(relation, &RowRel { row })
                .map_err(|err| UnpackError::new(err.to_string()))?;

            Ok(())
        }

        fn finish(self) -> Result<Self::Output, UnpackError> {
            self.s
                .end()
                .map_err(|err| UnpackError::new(err.to_string()))
        }
    }

    impl<T> Serialize for Row<T>
    where
        T: usql_core::Row,
        <T::Connector as Connector>::Error: core::error::Error + Send + Sync + 'static,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let ctx = RowSerializer {
                s: serializer.serialize_map(None)?,
            };

            self.unpack(ctx).map_err(|err| S::Error::custom(err))
        }
    }

    impl<R> Serialize for RowRel<R>
    where
        R: Unpack,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let ctx = RowSerializer {
                s: serializer.serialize_map(None)?,
            };

            Ok(self.row.unpack(ctx).unwrap())
        }
    }
}
