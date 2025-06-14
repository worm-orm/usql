use rusqlite::types::Value;

pub trait Params {
    fn into_params(self) -> Vec<Value>;
}

impl Params for Vec<Value> {
    fn into_params(self) -> Vec<Value> {
        self
    }
}

impl Params for () {
    fn into_params(self) -> Vec<Value> {
        Vec::default()
    }
}

impl Params for Vec<usql_value::Value> {
    fn into_params(self) -> Vec<Value> {
        self.into_iter().map(super::util::usql_to_sqlite).collect()
    }
}

impl<'a> Params for Vec<usql_value::ValueCow<'a>> {
    fn into_params(self) -> Vec<Value> {
        self.into_iter()
            .map(|m| m.to_owned())
            .map(super::util::usql_to_sqlite)
            .collect()
    }
}
