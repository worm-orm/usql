use usql::FromRow;

#[derive(Debug, FromRow)]
struct User {
    id: i32,
    name: String,
}

fn main() {}
