#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum System {
    Sqlite,
    LibSql,
    Postgres,
    Mysql,
}
