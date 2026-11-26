use futures::TryStreamExt;
use usql_core::{ColumnIndex, Connector, Executor, Row as _, Statement};
use usql_value::{Type, Value};

use crate::error::Error;

pub struct Row<B: Connector>(B::Row);

pub async fn query<E: Executor>(
    executor: &E,
    sql: &str,
    params: Vec<Value>,
) -> Result<Vec<Row<E::Connector>>, Error<E::Connector>> {
    let mut stmt = executor.prepare(sql).await.map_err(Error::Connector)?;
    let params = params.into_iter().map(Into::into).collect();
    let rows = executor
        .query(&mut stmt, params)
        .map_ok(Row)
        .try_collect()
        .await
        .map_err(Error::Connector)?;
    stmt.finalize().map_err(Error::Connector)?;
    Ok(rows)
}

impl<B: Connector> Row<B> {
    fn typed(&self, name: &'static str, ty: Type) -> Result<Value, Error<B>> {
        self.0
            .get_typed(ColumnIndex::Named(name), ty)
            .map(|m| m.to_owned())
            .map_err(Error::Connector)
    }

    pub fn opt(&self, name: &'static str) -> Result<Option<String>, Error<B>> {
        match self.typed(name, Type::Text)? {
            Value::Null => Ok(None),
            Value::Text(text) => Ok(Some(text.as_str().to_string())),
            _ => Err(Error::Value(name)),
        }
    }

    pub fn text(&self, name: &'static str) -> Result<String, Error<B>> {
        self.opt(name)?.ok_or(Error::Value(name))
    }

    pub fn int(&self, name: &'static str) -> Result<i32, Error<B>> {
        match self.typed(name, Type::Int)? {
            Value::Int(i) => Ok(i),
            _ => Err(Error::Value(name)),
        }
    }

    pub fn bool(&self, name: &'static str) -> Result<bool, Error<B>> {
        self.opt_bool(name)?.ok_or(Error::Value(name))
    }

    pub fn opt_bool(&self, name: &'static str) -> Result<Option<bool>, Error<B>> {
        match self.typed(name, Type::Bool)? {
            Value::Null => Ok(None),
            Value::Bool(b) => Ok(Some(b)),
            _ => Err(Error::Value(name)),
        }
    }

    pub fn opt_int(&self, name: &'static str) -> Result<Option<i32>, Error<B>> {
        match self.typed(name, Type::Int)? {
            Value::Null => Ok(None),
            Value::Int(i) => Ok(Some(i)),
            _ => Err(Error::Value(name)),
        }
    }

    pub fn list(&self, name: &'static str) -> Result<Vec<Option<String>>, Error<B>> {
        self.array(name, Type::Text, |value| match value {
            Value::Null => Some(None),
            Value::Text(text) => Some(Some(text.as_str().to_string())),
            _ => None,
        })
    }

    pub fn texts(&self, name: &'static str) -> Result<Vec<String>, Error<B>> {
        self.list(name)?
            .into_iter()
            .map(|m| m.ok_or(Error::Value(name)))
            .collect()
    }

    pub fn bools(&self, name: &'static str) -> Result<Vec<bool>, Error<B>> {
        self.array(name, Type::Bool, |value| match value {
            Value::Bool(b) => Some(b),
            _ => None,
        })
    }

    fn array<T>(
        &self,
        name: &'static str,
        ty: Type,
        func: impl Fn(Value) -> Option<T>,
    ) -> Result<Vec<T>, Error<B>> {
        match self.typed(name, Type::Array(Box::new(ty)))? {
            Value::Array(values) => values
                .into_iter()
                .map(|m| func(m).ok_or(Error::Value(name)))
                .collect(),
            _ => Err(Error::Value(name)),
        }
    }
}
