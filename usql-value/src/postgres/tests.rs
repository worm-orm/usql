use alloc::{string::ToString, vec, vec::Vec};
use bytes::BytesMut;
use geob::Geob;
use postgres_types::{FromSql, IsNull, Kind, ToSql, Type};

use super::numeric;
use crate::{JsonValue, Value};

fn encode(value: &Value, ty: &Type) -> Option<Vec<u8>> {
    let mut out = BytesMut::new();
    match value.to_sql_checked(ty, &mut out) {
        Ok(IsNull::No) => Some(out.to_vec()),
        Ok(IsNull::Yes) => Some(Vec::new()),
        Err(_) => None,
    }
}

fn round_trip(value: impl Into<Value>, ty: &Type) -> Value {
    let raw = encode(&value.into(), ty).expect("encode");
    Value::from_sql(ty, &raw).expect("decode")
}

#[test]
fn integers_follow_the_column() {
    assert_eq!(round_trip(7i32, &Type::INT8), Value::BigInt(7));
    assert_eq!(round_trip(7i64, &Type::INT2), Value::SmallInt(7));
    assert_eq!(round_trip(7i16, &Type::FLOAT8), Value::from(7.0f64));
    assert_eq!(round_trip(7i16, &Type::NUMERIC), Value::from("7"));
    assert!(encode(&Value::BigInt(40_000), &Type::INT2).is_none());
    assert!(encode(&Value::BigInt(-1), &Type::OID).is_none());
    assert!(encode(&Value::Int(1), &Type::TEXT).is_none());
    assert!(encode(&Value::from(f64::MAX), &Type::FLOAT4).is_none());
    assert!(encode(&Value::from(1.5f64), &Type::INT4).is_none());
}

#[test]
fn text_follows_the_column() {
    let id = uuid::Uuid::from_u128(42);
    assert_eq!(
        round_trip(id.to_string().as_str(), &Type::UUID),
        Value::Uuid(id)
    );
    assert_eq!(
        round_trip("[1]", &Type::JSONB),
        Value::Json(JsonValue::Array(vec![JsonValue::Integer(1)]))
    );
    let mood = Type::new(
        "mood".into(),
        50_001,
        Kind::Enum(vec!["ok".into()]),
        "public".into(),
    );
    assert_eq!(round_trip("ok", &mood), Value::from("ok"));
    let domain = Type::new(
        "age".into(),
        50_002,
        Kind::Domain(Type::INT4),
        "public".into(),
    );
    assert_eq!(round_trip(3i64, &domain), Value::Int(3));
    assert!(encode(&Value::from("x"), &Type::INT4).is_none());
}

#[test]
fn arrays() {
    let value = Value::Array(vec![Value::Int(1), Value::Null, Value::SmallInt(3)]);
    assert_eq!(
        round_trip(value.clone(), &Type::INT8_ARRAY),
        Value::Array(vec![Value::BigInt(1), Value::Null, Value::BigInt(3)])
    );
    assert!(encode(&value, &Type::TEXT_ARRAY).is_none());
    assert!(encode(&value, &Type::INT4).is_none());
    assert_eq!(
        round_trip(Value::Array(vec![]), &Type::TEXT_ARRAY),
        Value::Array(vec![])
    );
}

#[test]
fn unsupported_is_an_error() {
    assert!(Value::from_sql(&Type::INTERVAL, &[0; 16]).is_err());
    assert!(Value::from_sql(&Type::INT4, &[0]).is_err());
}

#[test]
fn geometry() {
    let ty = Type::new("geometry".into(), 50_000, Kind::Simple, "public".into());
    for text in [
        "POINT(1 2)",
        "SRID=4326;LINESTRING(1 2 3, 4 5 6)",
        "POLYGON((0 0, 1 0, 1 1, 0 0))",
        "MULTIPOINT((1 2), (3 4))",
        "MULTILINESTRING((1 2, 3 4))",
        "MULTIPOLYGON(((0 0, 1 0, 1 1, 0 0)))",
        "GEOMETRYCOLLECTION(POINT(1 2), GEOMETRYCOLLECTION(LINESTRING(1 2, 3 4)))",
        "POINT EMPTY",
    ] {
        let geo = Geob::from_text(text).unwrap();
        let value = Value::Geometry(geo.clone());
        let read = round_trip(value.clone(), &ty);
        let Value::Geometry(read) = read else {
            panic!("{text}")
        };
        assert_eq!(read.as_bytes(), geo.as_bytes(), "{text}");
        assert!(matches!(
            round_trip(value, &Type::BYTEA),
            Value::ByteArray(_)
        ));
    }
    // CIRCULARSTRING
    assert!(Value::from_sql(&ty, &[1, 8, 0, 0, 0, 0, 0, 0, 0]).is_err());
    assert!(encode(&Value::ByteArray(vec![1, 2].into()), &ty).is_none());
}

#[test]
fn numeric_round_trip() {
    for (input, expected) in [
        ("0", "0"),
        ("-0", "0"),
        ("0.00", "0.00"),
        ("1", "1"),
        ("-12345.678900", "-12345.678900"),
        ("10000", "10000"),
        ("100000000", "100000000"),
        ("0.0001", "0.0001"),
        ("0.00000001", "0.00000001"),
        (".5", "0.5"),
        ("5.", "5"),
        ("1.5e3", "1500"),
        ("1e-6", "0.000001"),
        ("12e-1", "1.2"),
        ("99999999999999999999.99999", "99999999999999999999.99999"),
        ("NaN", "NaN"),
        ("-inf", "-Infinity"),
    ] {
        let mut out = BytesMut::new();
        numeric::encode(input, &mut out).unwrap();
        assert_eq!(numeric::decode(&out).unwrap(), expected, "{input}");
    }
    for input in [
        "",
        "-",
        ".",
        "1.2.3",
        "abc",
        "1e",
        "1e999999",
        "1e-999999",
        "--1",
        "é",
    ] {
        assert!(
            numeric::encode(input, &mut BytesMut::new()).is_err(),
            "{input}"
        );
    }
    for raw in [&[][..], &[0, 1, 0, 0, 0, 0, 0, 0], &[0; 9]] {
        assert!(numeric::decode(raw).is_err());
    }
}
