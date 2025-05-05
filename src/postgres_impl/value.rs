use tokio_postgres::types::{IsNull, Json, ToSql};

use crate::{Value, ValueRef};

impl tokio_postgres::types::ToSql for ValueRef<'_> {
    fn to_sql(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<
        tokio_postgres::types::IsNull,
        alloc::boxed::Box<dyn core::error::Error + Sync + Send>,
    >
    where
        Self: Sized,
    {
        match self {
            ValueRef::Null => Ok(IsNull::Yes),
            ValueRef::Bool(b) => b.to_sql(ty, out),
            ValueRef::SmallInt(i) => i.to_sql(ty, out),
            ValueRef::Int(i) => i.to_sql(ty, out),
            ValueRef::BigInt(i) => i.to_sql(ty, out),
            ValueRef::Float(ordered_float) => ordered_float.to_sql(ty, out),
            ValueRef::Double(ordered_float) => ordered_float.to_sql(ty, out),
            ValueRef::Text(text) => text.to_sql(ty, out),
            ValueRef::ByteArray(items) => items.to_sql(ty, out),
            ValueRef::Date(naive_date) => naive_date.to_sql(ty, out),
            ValueRef::Time(naive_time) => naive_time.to_sql(ty, out),
            ValueRef::Timestamp(naive_date_time) => naive_date_time.to_sql(ty, out),
            ValueRef::Uuid(uuid) => uuid.to_sql(ty, out),
            ValueRef::Json(json_value) => Json(*json_value).to_sql(ty, out),
            ValueRef::Array(values) => values.to_sql(ty, out),
        }
    }

    fn accepts(ty: &tokio_postgres::types::Type) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn to_sql_checked(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<
        tokio_postgres::types::IsNull,
        alloc::boxed::Box<dyn core::error::Error + Sync + Send>,
    > {
        match self {
            ValueRef::Null => Ok(IsNull::Yes),
            ValueRef::Bool(b) => b.to_sql_checked(ty, out),
            ValueRef::SmallInt(i) => i.to_sql_checked(ty, out),
            ValueRef::Int(i) => i.to_sql_checked(ty, out),
            ValueRef::BigInt(i) => i.to_sql_checked(ty, out),
            ValueRef::Float(ordered_float) => ordered_float.to_sql_checked(ty, out),
            ValueRef::Double(ordered_float) => ordered_float.to_sql_checked(ty, out),
            ValueRef::Text(text) => text.to_sql_checked(ty, out),
            ValueRef::ByteArray(items) => items.to_sql_checked(ty, out),
            ValueRef::Date(naive_date) => naive_date.to_sql_checked(ty, out),
            ValueRef::Time(naive_time) => naive_time.to_sql_checked(ty, out),
            ValueRef::Timestamp(naive_date_time) => naive_date_time.to_sql_checked(ty, out),
            ValueRef::Uuid(uuid) => uuid.to_sql_checked(ty, out),
            ValueRef::Json(json_value) => Json(*json_value).to_sql_checked(ty, out),
            ValueRef::Array(values) => values.to_sql_checked(ty, out),
        }
    }
}

impl ToSql for Value {
    fn to_sql(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<IsNull, alloc::boxed::Box<dyn core::error::Error + Sync + Send>>
    where
        Self: Sized,
    {
        self.as_ref().to_sql(ty, out)
    }

    fn accepts(ty: &tokio_postgres::types::Type) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn to_sql_checked(
        &self,
        ty: &tokio_postgres::types::Type,
        out: &mut bytes::BytesMut,
    ) -> Result<IsNull, alloc::boxed::Box<dyn core::error::Error + Sync + Send>> {
        self.as_ref().to_sql_checked(ty, out)
    }
}
