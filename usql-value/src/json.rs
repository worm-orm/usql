use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use core::convert::TryFrom;
use ordered_float::OrderedFloat;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(OrderedFloat<f64>),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl core::fmt::Display for JsonValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JsonValue::Null => write!(f, "null"),
            JsonValue::Bool(b) => write!(f, "{}", b),
            JsonValue::Float(num) => write!(f, "{}", num),
            JsonValue::Integer(num) => write!(f, "{}", num),
            JsonValue::String(s) => write!(f, "\"{}\"", s),
            JsonValue::Array(arr) => {
                write!(f, "[")?;
                let mut first = true;
                for value in arr {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{}", value)?;
                }
                write!(f, "]")
            }
            JsonValue::Object(obj) => {
                write!(f, "{{")?;
                let mut first = true;
                for (key, value) in obj {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "\"{}\": {}", key, value)?;
                }
                write!(f, "}}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValueError {
    UnexpectedType(&'static str),
}

impl core::fmt::Display for JsonValueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            JsonValueError::UnexpectedType(msg) => write!(f, "Unexpected type: {}", msg),
        }
    }
}

impl core::error::Error for JsonValueError {}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        JsonValue::Bool(value)
    }
}

impl From<f64> for JsonValue {
    fn from(value: f64) -> Self {
        JsonValue::Float(OrderedFloat(value))
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        JsonValue::Integer(value)
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        JsonValue::String(value)
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        JsonValue::String(value.to_string())
    }
}

impl From<Vec<JsonValue>> for JsonValue {
    fn from(value: Vec<JsonValue>) -> Self {
        JsonValue::Array(value)
    }
}

impl From<BTreeMap<String, JsonValue>> for JsonValue {
    fn from(value: BTreeMap<String, JsonValue>) -> Self {
        JsonValue::Object(value)
    }
}

impl TryFrom<JsonValue> for bool {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::Bool(b) = value {
            Ok(b)
        } else {
            Err(JsonValueError::UnexpectedType("Expected JsonValue::Bool"))
        }
    }
}

impl TryFrom<JsonValue> for f64 {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::Float(f) = value {
            Ok(f.into_inner())
        } else {
            Err(JsonValueError::UnexpectedType("Expected JsonValue::Float"))
        }
    }
}

impl TryFrom<JsonValue> for i64 {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::Integer(i) = value {
            Ok(i)
        } else {
            Err(JsonValueError::UnexpectedType(
                "Expected JsonValue::Integer",
            ))
        }
    }
}

impl TryFrom<JsonValue> for String {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::String(s) = value {
            Ok(s)
        } else {
            Err(JsonValueError::UnexpectedType("Expected JsonValue::String"))
        }
    }
}

impl TryFrom<JsonValue> for Vec<JsonValue> {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::Array(arr) = value {
            Ok(arr)
        } else {
            Err(JsonValueError::UnexpectedType("Expected JsonValue::Array"))
        }
    }
}

impl TryFrom<JsonValue> for BTreeMap<String, JsonValue> {
    type Error = JsonValueError;

    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::Object(obj) = value {
            Ok(obj)
        } else {
            Err(JsonValueError::UnexpectedType("Expected JsonValue::Object"))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_display_null() {
        let value = JsonValue::Null;
        assert_eq!(value.to_string(), "null");
    }

    #[test]
    fn test_display_bool() {
        let value = JsonValue::Bool(true);
        assert_eq!(value.to_string(), "true");

        let value = JsonValue::Bool(false);
        assert_eq!(value.to_string(), "false");
    }

    #[test]
    fn test_display_float() {
        #[allow(clippy::approx_constant)]
        let value = JsonValue::Float(OrderedFloat(3.14));
        assert_eq!(value.to_string(), "3.14");
    }

    #[test]
    fn test_display_integer() {
        let value = JsonValue::Integer(42);
        assert_eq!(value.to_string(), "42");
    }

    #[test]
    fn test_display_string() {
        let value = JsonValue::String("hello".to_string());
        assert_eq!(value.to_string(), "\"hello\"");
    }

    #[test]
    fn test_display_array() {
        let value = JsonValue::Array(vec![
            JsonValue::Integer(1),
            JsonValue::Bool(false),
            JsonValue::String("test".to_string()),
        ]);
        assert_eq!(value.to_string(), "[1, false, \"test\"]");
    }

    #[test]
    fn test_display_object() {
        let mut map = BTreeMap::new();
        map.insert("key1".to_string(), JsonValue::Integer(10));
        map.insert("key2".to_string(), JsonValue::Bool(true));
        let value = JsonValue::Object(map);
        assert_eq!(value.to_string(), "{\"key1\": 10, \"key2\": true}");
    }

    #[test]
    fn test_display_nested() {
        let mut inner_map = BTreeMap::new();
        inner_map.insert(
            "inner_key".to_string(),
            JsonValue::String("value".to_string()),
        );

        let mut outer_map = BTreeMap::new();
        outer_map.insert(
            "array".to_string(),
            JsonValue::Array(vec![JsonValue::Null, JsonValue::Bool(true)]),
        );
        outer_map.insert("object".to_string(), JsonValue::Object(inner_map));

        let value = JsonValue::Object(outer_map);
        assert_eq!(
            value.to_string(),
            "{\"array\": [null, true], \"object\": {\"inner_key\": \"value\"}}"
        );
    }
}
