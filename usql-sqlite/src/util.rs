use usql_core::{Value, ValueRef};

pub fn sqlite_ref_to_usql(value: &rusqlite::types::Value) -> ValueRef<'_> {
    match value {
        rusqlite::types::Value::Null => ValueRef::Null,
        rusqlite::types::Value::Integer(i) => ValueRef::BigInt(*i),
        rusqlite::types::Value::Real(i) => ValueRef::Double((*i).into()),
        rusqlite::types::Value::Text(i) => ValueRef::Text(i),
        rusqlite::types::Value::Blob(items) => ValueRef::ByteArray(items),
    }
}

pub fn usql_to_sqlite(value: Value) -> rusqlite::types::Value {
    match value {
        Value::Bool(b) => rusqlite::types::Value::Integer((b).into()),
        Value::Text(s) => rusqlite::types::Value::Text(s),
        Value::Array(list) => {
            let string = serde_json::to_string(&list).expect("json encode");
            rusqlite::types::Value::Text(string)
        }
        Value::ByteArray(b) => rusqlite::types::Value::Blob(b.to_vec()),
        Value::Date(d) => rusqlite::types::Value::Text(d.format("%F").to_string()),
        Value::Timestamp(dt) => rusqlite::types::Value::Text(dt.format("%+").to_string()),
        Value::Time(t) => rusqlite::types::Value::Text(t.format("%T%.f").to_string()),
        Value::Uuid(u) => rusqlite::types::Value::Blob(u.as_bytes().to_vec()),
        Value::Int(n) => rusqlite::types::Value::Integer(n as _),
        Value::BigInt(n) => rusqlite::types::Value::Integer(n),
        Value::SmallInt(n) => rusqlite::types::Value::Integer(n as _),
        Value::Float(f) => rusqlite::types::Value::Real(*f as _),
        Value::Double(f) => rusqlite::types::Value::Real(*f),
        Value::Json(json) => {
            let string = serde_json::to_string(&json).expect("json encode");
            rusqlite::types::Value::Text(string)
        }
        Value::Null => rusqlite::types::Value::Null,
    }
}
