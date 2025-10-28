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

#[cfg(feature = "bycat")]
#[derive(Debug, Clone, Copy)]
pub struct Bycat;

#[cfg(feature = "bycat")]
impl Output for Bycat {
    type Writer = BycatWriter;
    fn create(&self) -> Self::Writer {
        BycatWriter::default()
    }
}

#[cfg(feature = "bycat")]
#[derive(Default)]
pub struct BycatWriter {
    output: bycat_value::Map,
}

#[cfg(feature = "bycat")]
impl RowWriter for BycatWriter {
    type Output = bycat_value::Value;

    fn set_value(&mut self, field: &str, value: ValueCow<'_>) -> Result<(), UnpackError> {
        let value: bycat_value::Value = value
            .to_owned()
            .try_into()
            .map_err(|err| UnpackError::new(err))?;
        self.output.insert(field, value);
        Ok(())
    }

    fn set_many<I: Iterator>(&mut self, field: &str, many: I) -> Result<(), UnpackError>
    where
        I::Item: Unpack,
    {
        let mut children: bycat_value::List<bycat_value::Value> = bycat_value::List::default();

        for item in many {
            let value = item.unpack(Self::default())?;
            children.push(value)
        }

        self.output.insert(field, children);

        Ok(())
    }

    fn set_one<O: Unpack>(&mut self, field: &str, one: O) -> Result<(), UnpackError> {
        self.output.insert(field, one.unpack(Self::default())?);
        Ok(())
    }

    fn finish(self) -> Result<Self::Output, UnpackError> {
        Ok(bycat_value::Value::Map(self.output))
    }
}
