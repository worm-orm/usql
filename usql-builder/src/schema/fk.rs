use core::fmt;

use alloc::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ForeignKey<'a> {
    pub table: Cow<'a, str>,
    pub column: Cow<'a, str>,
    pub on_update: ReferentialAction,
    pub on_delete: ReferentialAction,
}

impl<'a> ForeignKey<'a> {
    pub fn new(table: impl Into<Cow<'a, str>>, column: impl Into<Cow<'a, str>>) -> ForeignKey<'a> {
        ForeignKey {
            table: table.into(),
            column: column.into(),
            on_delete: ReferentialAction::NoAction,
            on_update: ReferentialAction::NoAction,
        }
    }

    pub fn on_delete(mut self, action: ReferentialAction) -> Self {
        self.on_delete = action;
        self
    }

    pub fn on_update(mut self, action: ReferentialAction) -> Self {
        self.on_update = action;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferentialAction {
    Cascade,
    Restrict,
    SetNull,
    SetDefault,
    NoAction,
}

impl fmt::Display for ReferentialAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cascade => write!(f, "CASCADE"),
            Self::Restrict => write!(f, "RESTRICT"),
            Self::SetDefault => write!(f, "SET DEFAULT"),
            Self::SetNull => write!(f, "SET NULL"),
            Self::NoAction => write!(f, "NO ACTION"),
        }
    }
}
