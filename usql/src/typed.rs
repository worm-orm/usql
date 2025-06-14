use alloc::string::String;
use usql_value::Type;
use usql_value::chrono::{NaiveDate, NaiveDateTime, NaiveTime};

pub trait Typed {
    const TYPE: Type;
}

macro_rules! types {
    ($($ty: ty => $variant: ident),+) => {
      $(
        impl Typed for $ty {
          const TYPE: Type = Type::$variant;
        }
      )*
    };
}

types!(
  u8 => SmallInt,
  i8 => SmallInt,
  u16 => SmallInt,
  i16 => SmallInt,
  i32 => Int,
  u32 => Int,
  u64 => BigInt,
  i64 => BigInt,
  bool => Bool,
  String => Text,
  NaiveDate => Date,
  NaiveDateTime => DateTime,
  NaiveTime => Time
);
