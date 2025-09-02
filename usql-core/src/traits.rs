use crate::system::System;
use alloc::vec::Vec;
use futures_core::stream::BoxStream;
use usql_value::{Type, ValueCow};

pub trait Connector: Send + Sync {
    type Connection: Connection<Connector = Self>;
    type Statement: Statement<Connector = Self>;
    type Row: Row<Connector = Self>;
    type Info: DatabaseInfo;
    type Pool: Pool<Connector = Self>;
    type Error;
    type Options;

    fn create_pool(
        options: Self::Options,
    ) -> impl Future<Output = Result<Self::Pool, Self::Error>> + Send;
}

pub trait Pool: Send + Sync {
    type Connector: Connector;
    fn get(
        &self,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as Connector>::Connection,
            <Self::Connector as Connector>::Error,
        >,
    > + Send
    + '_;
}

pub trait DatabaseInfo {
    fn variant(&self) -> System;
}

pub type QueryStream<'a, P> = BoxStream<'a, Result<<P as Connector>::Row, <P as Connector>::Error>>;

pub trait Executor {
    type Connector: Connector;

    fn db_info(&self) -> <Self::Connector as Connector>::Info;

    fn prepare<'a>(
        &'a self,
        query: &'a str,
    ) -> impl Future<
        Output = Result<
            <Self::Connector as Connector>::Statement,
            <Self::Connector as Connector>::Error,
        >,
    > + Send
    + 'a;

    fn query<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> QueryStream<'a, Self::Connector>;

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<ValueCow<'a>>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a;

    fn exec_batch<'a>(
        &'a self,
        stmt: &'a str,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a;
}

pub trait Connection: Executor + Send + Sync {
    type Transaction<'conn>: Transaction<'conn, Connector = Self::Connector>
    where
        Self: 'conn;

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, <Self::Connector as Connector>::Error>> + Send;
}

pub trait Transaction<'conn>: Executor {
    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send;

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send;
}

pub trait Statement: Send + Sync {
    type Connector: Connector;

    fn finalize(self) -> Result<(), <Self::Connector as Connector>::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnIndex<'a> {
    Named(&'a str),
    Index(usize),
}

impl<'a> ColumnIndex<'a> {
    pub fn named(input: &'a str) -> ColumnIndex<'a> {
        ColumnIndex::Named(input.into())
    }
}

impl<'a> From<&'a str> for ColumnIndex<'a> {
    fn from(value: &'a str) -> Self {
        ColumnIndex::Named(value)
    }
}

impl<'a> From<usize> for ColumnIndex<'a> {
    fn from(value: usize) -> Self {
        ColumnIndex::Index(value)
    }
}

impl<'a> From<u64> for ColumnIndex<'a> {
    fn from(value: u64) -> Self {
        ColumnIndex::Index(value as _)
    }
}

impl<'a> From<i32> for ColumnIndex<'a> {
    fn from(value: i32) -> Self {
        ColumnIndex::Index(value as _)
    }
}

pub trait Row: Send {
    type Connector: Connector;
    fn get<'a>(
        &'a self,
        index: ColumnIndex<'_>,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error>;

    fn get_typed<'a>(
        &'a self,
        index: ColumnIndex<'_>,
        ty: Type,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn column_name(&self, idx: usize) -> Option<&str>;
}
