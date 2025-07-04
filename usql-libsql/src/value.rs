use std::string::ToString;

use libsql::params::IntoValue;

use usql_core::Value;

impl IntoValue for Value {
    fn into_value(self) -> libsql::Result<libsql::Value> {
        let ret = match self {
            Value::Bool(b) => libsql::Value::Integer((b).into()),
            Value::Text(s) => libsql::Value::Text(s),
            Value::Array(list) => {
                let string = serde_json::to_string(&list).expect("json encode");
                libsql::Value::Text(string)
            }
            Value::ByteArray(b) => libsql::Value::Blob(b.to_vec()),
            Value::Date(d) => libsql::Value::Text(d.format("%F").to_string()),
            Value::Timestamp(dt) => libsql::Value::Text(dt.format("%+").to_string()),
            Value::Time(t) => libsql::Value::Text(t.format("%T%.f").to_string()),
            Value::Uuid(u) => libsql::Value::Blob(u.as_bytes().to_vec()),
            Value::Int(n) => libsql::Value::Integer(n as _),
            Value::BigInt(n) => libsql::Value::Integer(n),
            Value::SmallInt(n) => libsql::Value::Integer(n as _),
            Value::Float(f) => libsql::Value::Real(*f as _),
            Value::Double(f) => libsql::Value::Real(*f),
            Value::Json(json) => {
                let string = serde_json::to_string(&json).expect("json encode");
                libsql::Value::Text(string)
            }
            Value::Null => libsql::Value::Null,
        };

        Ok(ret)
    }
}
