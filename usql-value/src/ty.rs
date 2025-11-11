use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Text,
    SmallInt,
    BigInt,
    Int,
    Blob,
    Time,
    Date,
    DateTime,
    Json,
    Float,
    Double,
    Uuid,
    Bool,
    Array(Box<Type>),
    Geometry,
    Any,
}

impl Type {
    pub fn is_text(&self) -> bool {
        matches!(self, Type::Text)
    }

    pub fn is_small_int(&self) -> bool {
        matches!(self, Type::SmallInt)
    }

    pub fn is_big_int(&self) -> bool {
        matches!(self, Type::BigInt)
    }

    pub fn is_blob(&self) -> bool {
        matches!(self, Type::Blob)
    }

    pub fn is_time(&self) -> bool {
        matches!(self, Type::Time)
    }

    pub fn is_date(&self) -> bool {
        matches!(self, Type::Date)
    }

    pub fn is_date_time(&self) -> bool {
        matches!(self, Type::DateTime)
    }

    pub fn is_json(&self) -> bool {
        matches!(self, Type::Json)
    }

    pub fn is_real(&self) -> bool {
        matches!(self, Type::Float)
    }

    pub fn is_double(&self) -> bool {
        matches!(self, Type::Double)
    }

    pub fn is_uuid(&self) -> bool {
        matches!(self, Type::Uuid)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Type::Bool)
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Type::Array(_))
    }

    pub fn is_geometry(&self) -> bool {
        matches!(self, Type::Geometry)
    }
}

impl core::fmt::Display for Type {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Type::Text => write!(f, "text"),
            Type::SmallInt => write!(f, "small-int"),
            Type::BigInt => write!(f, "big-int"),
            Type::Int => write!(f, "integer"),
            Type::Blob => write!(f, "blob"),
            Type::Time => write!(f, "time"),
            Type::Date => write!(f, "date"),
            Type::DateTime => write!(f, "date-time"),
            Type::Json => write!(f, "json"),
            Type::Float => write!(f, "float"),
            Type::Double => write!(f, "double"),
            Type::Uuid => write!(f, "uuid"),
            Type::Bool => write!(f, "bool"),
            Type::Array(inner) => write!(f, "{}[]", inner),
            Type::Geometry => write!(f, "geometry"),
            Type::Any => write!(f, "any"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTypeError {
    details: String,
}

impl ParseTypeError {
    fn new(msg: &str) -> Self {
        ParseTypeError {
            details: msg.to_string(),
        }
    }
}

impl core::fmt::Display for ParseTypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl core::error::Error for ParseTypeError {}

impl core::str::FromStr for Type {
    type Err = ParseTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Type::Text),
            "small-int" => Ok(Type::SmallInt),
            "big-int" => Ok(Type::BigInt),
            "blob" => Ok(Type::Blob),
            "time" => Ok(Type::Time),
            "date" => Ok(Type::Date),
            "date-time" => Ok(Type::DateTime),
            "json" => Ok(Type::Json),
            "real" => Ok(Type::Float),
            "double" => Ok(Type::Double),
            "uuid" => Ok(Type::Uuid),
            "bool" => Ok(Type::Bool),
            "geometry" => Ok(Type::Geometry),
            s if s.ends_with("[]") => {
                let inner = &s[..s.len() - 2];
                inner
                    .parse()
                    .map(|t| Type::Array(Box::new(t)))
                    .map_err(|e| ParseTypeError::new(&e.to_string()))
            }
            _ => Err(ParseTypeError::new(&format!("Unknown type: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn test_type_display() {
        assert_eq!(Type::Text.to_string(), "text");
        assert_eq!(Type::SmallInt.to_string(), "small-int");
        assert_eq!(Type::BigInt.to_string(), "big-int");
        assert_eq!(Type::Blob.to_string(), "blob");
        assert_eq!(Type::Time.to_string(), "time");
        assert_eq!(Type::Date.to_string(), "date");
        assert_eq!(Type::DateTime.to_string(), "date-time");
        assert_eq!(Type::Json.to_string(), "json");
        assert_eq!(Type::Float.to_string(), "real");
        assert_eq!(Type::Double.to_string(), "double");
        assert_eq!(Type::Uuid.to_string(), "uuid");
        assert_eq!(Type::Bool.to_string(), "bool");
        assert_eq!(Type::Array(Box::new(Type::Text)).to_string(), "text[]");
    }

    #[test]
    fn test_type_from_str() {
        assert_eq!(Type::from_str("text").unwrap(), Type::Text);
        assert_eq!(Type::from_str("small-int").unwrap(), Type::SmallInt);
        assert_eq!(Type::from_str("big-int").unwrap(), Type::BigInt);
        assert_eq!(Type::from_str("blob").unwrap(), Type::Blob);
        assert_eq!(Type::from_str("time").unwrap(), Type::Time);
        assert_eq!(Type::from_str("date").unwrap(), Type::Date);
        assert_eq!(Type::from_str("date-time").unwrap(), Type::DateTime);
        assert_eq!(Type::from_str("json").unwrap(), Type::Json);
        assert_eq!(Type::from_str("real").unwrap(), Type::Float);
        assert_eq!(Type::from_str("double").unwrap(), Type::Double);
        assert_eq!(Type::from_str("uuid").unwrap(), Type::Uuid);
        assert_eq!(Type::from_str("bool").unwrap(), Type::Bool);
        assert_eq!(
            Type::from_str("text[]").unwrap(),
            Type::Array(Box::new(Type::Text))
        );
        assert_eq!(
            Type::from_str("big-int[]").unwrap(),
            Type::Array(Box::new(Type::BigInt))
        );
    }

    #[test]
    fn test_type_from_str_invalid() {
        assert!(Type::from_str("unknown").is_err());
        assert!(Type::from_str("text[]]").is_err());
        assert!(Type::from_str("[]").is_err());
        // assert!(Type::from_str("text[][]").is_err());
    }

    // #[test]
    // fn test_parse_type_error_display() {
    //     let error = ParseTypeError::new("Invalid type");
    //     assert_eq!(error.to_string(), "Invalid type");
    // }

    // #[test]
    // fn test_parse_type_error_debug() {
    //     let error = ParseTypeError::new("Invalid type");
    //     assert_eq!(format!("{:?}", error), "ParseTypeError: Invalid type");
    // }
}
