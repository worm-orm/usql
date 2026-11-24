use alloc::{
    format,
    string::{String, ToString},
};
use bytes::{BufMut, BytesMut};
use geob::{Geob, GeobRef};
use postgres_types::{IsNull, Json, Kind, ToSql, Type};

use super::{BoxError, numeric};
use crate::{Value, ValueRef};

impl ToSql for ValueRef<'_> {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
        encode(*self, ty, out)
    }

    // What is accepted depends on the value, so it is checked when encoding
    fn accepts(_ty: &Type) -> bool {
        true
    }

    fn to_sql_checked(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
        encode(*self, ty, out)
    }
}

impl ToSql for Value {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
        encode(self.as_ref(), ty, out)
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    fn to_sql_checked(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
        encode(self.as_ref(), ty, out)
    }
}

fn encode(value: ValueRef<'_>, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
    if let Kind::Domain(inner) = ty.kind() {
        return encode(value, inner, out);
    }

    match value {
        ValueRef::Null => return Ok(IsNull::Yes),
        ValueRef::Bool(b) if *ty == Type::BOOL => return b.to_sql(ty, out),
        ValueRef::SmallInt(i) => return encode_int(i.into(), ty, out),
        ValueRef::Int(i) => return encode_int(i.into(), ty, out),
        ValueRef::BigInt(i) => return encode_int(i, ty, out),
        ValueRef::Float(f) => match *ty {
            Type::FLOAT4 => return f.0.to_sql(ty, out),
            Type::FLOAT8 => return f64::from(f.0).to_sql(ty, out),
            Type::NUMERIC => return encode_numeric(&f.0.to_string(), out),
            _ => {}
        },
        ValueRef::Double(f) => match *ty {
            Type::FLOAT4 => {
                let narrow = f.0 as f32;
                if narrow.is_infinite() && f.0.is_finite() {
                    return Err(out_of_range(f.0, ty));
                }
                return narrow.to_sql(ty, out);
            }
            Type::FLOAT8 => return f.0.to_sql(ty, out),
            Type::NUMERIC => return encode_numeric(&f.0.to_string(), out),
            _ => {}
        },
        ValueRef::Text(text) => match *ty {
            Type::JSON | Type::XML => {
                out.put_slice(text.as_bytes());
                return Ok(IsNull::No);
            }
            Type::JSONB => {
                // Version of the format
                out.put_u8(1);
                out.put_slice(text.as_bytes());
                return Ok(IsNull::No);
            }
            Type::UUID => return uuid::Uuid::parse_str(text)?.to_sql(ty, out),
            Type::NUMERIC => return encode_numeric(text, out),
            _ if matches!(ty.kind(), Kind::Enum(_)) => {
                out.put_slice(text.as_bytes());
                return Ok(IsNull::No);
            }
            _ if <&str as ToSql>::accepts(ty) => return text.to_sql(ty, out),
            _ => {}
        },
        ValueRef::ByteArray(bytes) => {
            if *ty == Type::BYTEA {
                return bytes.to_sql(ty, out);
            }
            if <Geob as ToSql>::accepts(ty) {
                return GeobRef::from_bytes(bytes)?.to_sql(ty, out);
            }
        }
        ValueRef::Date(date) if *ty == Type::DATE => return date.to_sql(ty, out),
        ValueRef::Time(time) if *ty == Type::TIME => return time.to_sql(ty, out),
        ValueRef::Timestamp(time) => match *ty {
            Type::TIMESTAMP => return time.to_sql(ty, out),
            Type::TIMESTAMPTZ => return time.and_utc().to_sql(ty, out),
            _ => {}
        },
        ValueRef::Uuid(uuid) if *ty == Type::UUID => return uuid.to_sql(ty, out),
        ValueRef::Json(json) if matches!(*ty, Type::JSON | Type::JSONB) => {
            return Json(json).to_sql(ty, out);
        }
        ValueRef::Array(values) => match ty.kind() {
            Kind::Array(_) => return values.to_sql(ty, out),
            _ if matches!(*ty, Type::JSON | Type::JSONB) => return Json(values).to_sql(ty, out),
            _ => {}
        },
        ValueRef::Geometry(geo) if <Geob as ToSql>::accepts(ty) => return geo.to_sql(ty, out),
        _ => {}
    }

    Err(format!("cannot convert {} to the postgres type {ty}", name(value)).into())
}

fn encode_int(value: i64, ty: &Type, out: &mut BytesMut) -> Result<IsNull, BoxError> {
    match *ty {
        Type::INT2 => i16::try_from(value)
            .map_err(|_| out_of_range(value, ty))?
            .to_sql(ty, out),
        Type::INT4 => i32::try_from(value)
            .map_err(|_| out_of_range(value, ty))?
            .to_sql(ty, out),
        Type::INT8 => value.to_sql(ty, out),
        Type::OID => u32::try_from(value)
            .map_err(|_| out_of_range(value, ty))?
            .to_sql(ty, out),
        Type::FLOAT4 => (value as f32).to_sql(ty, out),
        Type::FLOAT8 => (value as f64).to_sql(ty, out),
        Type::NUMERIC => encode_numeric(&value.to_string(), out),
        _ => Err(format!("cannot convert integer to the postgres type {ty}").into()),
    }
}

fn encode_numeric(value: &str, out: &mut BytesMut) -> Result<IsNull, BoxError> {
    numeric::encode(value, out)?;
    Ok(IsNull::No)
}

fn out_of_range(value: impl core::fmt::Display, ty: &Type) -> BoxError {
    format!("{value} is out of range for the postgres type {ty}").into()
}

fn name(value: ValueRef<'_>) -> String {
    match value.get_type() {
        Some(ty) => ty.to_string(),
        None => "value".to_string(),
    }
}
