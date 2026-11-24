use usql_builder::schema::ReferentialAction;
use usql_core::System;
use usql_value::Type;

macro_rules! model {
    ($($(#[$meta: meta])* pub struct $name: ident { $($(#[$fmeta: meta])* pub $field: ident: $ty: ty,)* })*) => {
        $(
            $(#[$meta])*
            #[derive(Debug, Clone, PartialEq)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize))]
            #[non_exhaustive]
            pub struct $name {
                $($(#[$fmeta])* pub $field: $ty,)*
            }
        )*
    };
}

model! {
    pub struct Schema {
        pub name: String,
        pub tables: Vec<Table>,
        pub views: Vec<View>,
        pub enums: Vec<Enum>,
    }

    /// A table without its contents
    pub struct TableName {
        pub schema: String,
        pub name: String,
        pub kind: TableKind,
    }

    pub struct Table {
        /// The kind of database the table was read from
        #[cfg_attr(feature = "serde", serde(skip))]
        pub system: System,
        pub schema: String,
        pub name: String,
        pub kind: TableKind,
        pub comment: Option<String>,
        /// The table this is a partition of
        pub partition_of: Option<String>,
        /// `STRICT` (SQLite)
        pub strict: bool,
        /// `WITHOUT ROWID` (SQLite)
        pub without_rowid: bool,
        pub columns: Vec<Column>,
        pub primary_key: Option<Key>,
        pub foreign_keys: Vec<ForeignKey>,
        pub uniques: Vec<Key>,
        /// Excludes those that implement the primary key and unique constraints
        pub indexes: Vec<Index>,
        pub triggers: Vec<Trigger>,
    }

    pub struct Column {
        pub name: String,
        /// Starts at 1, and may have gaps
        pub position: i32,
        /// The type as written by the database
        pub type_name: String,
        /// The closest type, if there is one
        #[cfg_attr(feature = "serde", serde(serialize_with = "ser::ty"))]
        pub ty: Option<Type>,
        pub nullable: bool,
        /// An SQL expression
        pub default: Option<String>,
        pub auto: Option<Auto>,
        pub generated: Option<Generated>,
        pub comment: Option<String>,
    }

    pub struct Generated {
        /// Not known on SQLite
        pub expression: Option<String>,
        pub stored: bool,
    }

    /// A primary key or a unique constraint
    pub struct Key {
        /// Not known on SQLite
        pub name: Option<String>,
        pub columns: Vec<String>,
    }

    pub struct ForeignKey {
        /// Not known on SQLite
        pub name: Option<String>,
        pub columns: Vec<String>,
        pub ref_schema: String,
        pub ref_table: String,
        pub ref_columns: Vec<String>,
        #[cfg_attr(feature = "serde", serde(serialize_with = "ser::display"))]
        pub on_update: ReferentialAction,
        #[cfg_attr(feature = "serde", serde(serialize_with = "ser::display"))]
        pub on_delete: ReferentialAction,
    }

    pub struct Index {
        pub name: String,
        pub unique: bool,
        pub method: String,
        pub parts: Vec<IndexPart>,
        pub included: Vec<String>,
        pub partial: bool,
        /// Only known on Postgres
        pub predicate: Option<String>,
        pub definition: Option<String>,
    }

    /// Either a column or an expression
    pub struct IndexPart {
        pub column: Option<String>,
        /// Not known on SQLite
        pub expression: Option<String>,
        pub descending: bool,
        /// When it is not the default. Only known on SQLite
        pub collation: Option<String>,
    }

    pub struct View {
        pub schema: String,
        pub name: String,
        pub materialized: bool,
        pub comment: Option<String>,
        pub columns: Vec<Column>,
        pub triggers: Vec<Trigger>,
        pub definition: Option<String>,
    }

    pub struct Trigger {
        pub name: String,
        pub definition: String,
    }

    pub struct Enum {
        pub schema: String,
        pub name: String,
        pub values: Vec<String>,
        pub comment: Option<String>,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[non_exhaustive]
pub enum TableKind {
    Ordinary,
    Partitioned,
    Virtual,
    Foreign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[non_exhaustive]
pub enum Auto {
    Serial,
    Identity,
    AutoIncrement,
}

#[cfg(feature = "serde")]
mod ser {
    use serde::Serializer;
    use usql_value::Type;

    pub fn display<T: core::fmt::Display, S: Serializer>(
        value: &T,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        s.collect_str(value)
    }

    pub fn ty<S: Serializer>(value: &Option<Type>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(ty) => s.collect_str(ty),
            None => s.serialize_none(),
        }
    }
}
