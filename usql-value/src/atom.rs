use alloc::{
    borrow::Cow,
    fmt,
    string::{String, ToString},
    sync::Arc,
};
use core::{borrow::Borrow, hash::Hash};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialOrd, Ord, Clone)]
pub struct Atom(Arc<String>);

impl Atom {
    pub fn new(input: impl AsRef<str>) -> Atom {
        Atom(Arc::from(input.as_ref().to_string()))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Atom {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl Eq for Atom {}

impl PartialEq<&str> for Atom {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<str> for Atom {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl Hash for Atom {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl Borrow<str> for Atom {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Atom {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::ops::Deref for Atom {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl From<String> for Atom {
    fn from(value: String) -> Self {
        Atom(value.into())
    }
}

impl<'a> From<&'a str> for Atom {
    fn from(value: &'a str) -> Self {
        Atom(value.to_string().into())
    }
}

impl From<Arc<String>> for Atom {
    fn from(value: Arc<String>) -> Self {
        Atom(value)
    }
}

impl<'a> From<&'a Atom> for Cow<'a, str> {
    fn from(value: &'a Atom) -> Self {
        Cow::Borrowed(value.as_str())
    }
}

impl From<Atom> for Cow<'_, str> {
    fn from(value: Atom) -> Self {
        Cow::Owned(value.into())
    }
}

impl From<Atom> for String {
    fn from(value: Atom) -> Self {
        Arc::unwrap_or_clone(value.0)
    }
}
