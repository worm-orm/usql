use alloc::vec::Vec;
use futures_core::stream::BoxStream;

use crate::{ValueCow, system::System, value::Value};

pub trait Connector: Send + Sync {
    type Connection: Connection<Connector = Self>;
    type Statement: Statement<Connector = Self>;
    type Row: Row<Connector = Self>;
    type Info: DatabaseInfo;
    type Pool: Pool;
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

pub trait Connection: Send + Sync {
    type Connector: Connector;

    type Transaction<'conn>: Transaction<'conn, Connector = Self::Connector>
    where
        Self: 'conn;

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
        params: Vec<Value>,
    ) -> QueryStream<'a, Self::Connector>;

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<Value>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a;

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Self::Transaction<'_>, <Self::Connector as Connector>::Error>> + Send;
}

pub trait Transaction<'conn> {
    type Connector: Connector;

    fn commit(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send;

    fn rollback(
        self,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send;

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
        params: Vec<Value>,
    ) -> QueryStream<'a, Self::Connector>;

    fn exec<'a>(
        &'a self,
        stmt: &'a mut <Self::Connector as Connector>::Statement,
        params: Vec<Value>,
    ) -> impl Future<Output = Result<(), <Self::Connector as Connector>::Error>> + Send + 'a;
}

pub trait Statement: Send + Sync {
    type Connector: Connector;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColumnIndex<'a> {
    Named(&'a str),
    Index(usize),
}

pub trait Row: Send {
    type Connector: Connector;
    fn get<'a>(
        &'a self,
        index: ColumnIndex<'_>,
    ) -> Result<ValueCow<'a>, <Self::Connector as Connector>::Error>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
