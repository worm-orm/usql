use super::{Value, ValueRef};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueCow<'a> {
    Ref(ValueRef<'a>),
    Owned(Value),
}

impl ValueCow<'_> {
    pub fn as_ref(&self) -> ValueRef<'_> {
        match self {
            ValueCow::Owned(v) => v.as_ref(),
            ValueCow::Ref(v) => *v,
        }
    }

    pub fn to_owned(self) -> Value {
        match self {
            Self::Ref(i) => i.into(),
            Self::Owned(i) => i,
        }
    }
}

impl From<Value> for ValueCow<'_> {
    fn from(value: Value) -> Self {
        ValueCow::Owned(value)
    }
}

impl<'a> From<ValueRef<'a>> for ValueCow<'a> {
    fn from(value: ValueRef<'a>) -> Self {
        ValueCow::Ref(value)
    }
}

impl<'a> From<ValueCow<'a>> for Value {
    fn from(value: ValueCow<'a>) -> Self {
        value.to_owned()
    }
}
