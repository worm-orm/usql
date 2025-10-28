mod delete;
mod insert;
mod insert_many;
mod set;
mod update;

pub use self::{
    delete::*,
    insert::*,
    insert_many::*,
    set::{Returning, ReturningStmt, Set},
    update::*,
};
