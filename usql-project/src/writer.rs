use std::collections::HashMap;

use crate::UnpackError;
use usql_value::{Value, ValueCow};

pub trait Unpack {
    fn unpack<W: RowWriter>(&self, writer: W) -> Result<W::Output, UnpackError>;
}

impl<'a, T> Unpack for &'a T
where
    T: Unpack,
{
    fn unpack<W: RowWriter>(&self, writer: W) -> Result<W::Output, UnpackError> {
        (**self).unpack(writer)
    }
}

pub trait Output {
    type Writer: RowWriter;
    fn create(&self) -> Self::Writer;
}

pub trait RowWriter {
    type Output;
    fn set_value(&mut self, field: &str, value: ValueCow<'_>) -> Result<(), UnpackError>;
    fn set_many<I: Iterator>(&mut self, field: &str, many: I) -> Result<(), UnpackError>
    where
        I::Item: Unpack;
    fn set_one<O: Unpack>(&mut self, field: &str, one: O) -> Result<(), UnpackError>;

    fn finish(self) -> Result<Self::Output, UnpackError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultOutput;

impl Output for DefaultOutput {
    type Writer = DefaultWriter;

    fn create(&self) -> Self::Writer {
        DefaultWriter::default()
    }
}

#[derive(Default)]
pub struct DefaultWriter {
    i: HashMap<String, DefaultValue>,
}

impl RowWriter for DefaultWriter {
    type Output = DefaultValue;

    fn set_value(&mut self, field: &str, value: ValueCow<'_>) -> Result<(), UnpackError> {
        self.i
            .insert(field.to_string(), DefaultValue::Scalar(value.to_owned()));
        Ok(())
    }

    fn set_many<I: Iterator>(&mut self, field: &str, many: I) -> Result<(), UnpackError>
    where
        I::Item: Unpack,
    {
        let mut list = Vec::new();

        for item in many {
            let writer = Self::default();
            list.push(item.unpack(writer)?);
        }

        self.i.insert(field.to_string(), DefaultValue::List(list));

        Ok(())
    }

    fn set_one<O: Unpack>(&mut self, field: &str, one: O) -> Result<(), UnpackError> {
        let writer = DefaultWriter::default();
        let output = one.unpack(writer)?;
        self.i.insert(field.to_string(), output);
        Ok(())
    }

    fn finish(self) -> Result<Self::Output, UnpackError> {
        Ok(DefaultValue::Map(self.i))
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum DefaultValue {
    Map(HashMap<String, DefaultValue>),
    List(Vec<DefaultValue>),
    Scalar(Value),
}
