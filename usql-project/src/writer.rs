use usql_value::ValueCow;

pub trait Unpack {
    fn unpack<W: RowWriter>(&self, writer: W) -> Result<W::Output, W::Error>;
}

pub trait RowWriter {
    type Error;
    type Output;
    fn set_value(&mut self, field: &str, value: ValueCow<'_>) -> Result<(), Self::Error>;
    fn set_many<I: Iterator>(&mut self, field: &str, many: I) -> Result<(), Self::Error>
    where
        I::Item: Unpack;
    fn set_one<O: Unpack>(&mut self, field: &str, one: O) -> Result<(), Self::Error>;

    fn finish(self) -> Result<Self::Output, Self::Error>;
}
