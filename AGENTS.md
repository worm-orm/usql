# AGENTS.md

Guidance for coding agents working in this repository.

usql is a Rust (edition 2024) Cargo workspace: an async, driver-agnostic SQL
toolkit with a query builder, value types, migrations and row projection.

## Crates

| Crate | Purpose |
| --- | --- |
| `usql-value` | `Value` / `ValueRef` / `ValueCow`, `Type`, conversions. `no_std` + `alloc`. |
| `usql-core` | Driver traits: `Connector`, `Pool`, `Connection`, `Executor`, `Transaction`, `Statement`, `Row`. Also `System` (SQL dialect). `no_std` + `alloc`. |
| `usql-builder` | Query, mutation and schema builder. `no_std` + `alloc`. |
| `usql` | User-facing facade (`Pool`, `Conn`, `Trans`, `Row`, `FromRow`). Re-exports builder, core and value. `no_std` + `alloc`. |
| `usql-macros` | `#[derive(FromRow)]`; enabled by `usql`'s default `derive` feature. |
| `usql-sqlite` | SQLite driver (rusqlite). Features: `bundled`, `vector`, `geometry`. |
| `usql-postgres` | PostgreSQL driver (tokio-postgres, deadpool). Needs a Tokio runtime. Feature: `rustls`. |
| `usql-mysql` | MySQL driver (mysql_async, its own pool). Needs a Tokio runtime. Feature: `rustls`. |
| `usql-any` | Runtime-selected driver; backends are feature-gated (`sqlite`, `sqlite-vector`, `postgres`, `mysql`). |
| `usql-migrate` | Migrations and loaders; SQL file loader behind `sql`. |
| `usql-project` | Row projection into nested results; `serde` and `bycat` features. |
| `usql-util` | Utilities, including another projection implementation. |
| `usql-quick` | QuickJS bindings (rquickjs, klaver). Types in `usql.d.ts`. |
| `usql-schema` | Reads metadata (tables, keys, indexes, views, triggers, enums) with `Inspector`, and converts tables to builder statements. Feature: `serde`. |
| `usql-cli` | The `usql` binary. |
| `usql-libsql` | libSQL driver. **Not a workspace member**, so it is not compiled. |

## Commands

```sh
cargo check -p <crate>                        # prefer per-crate checks
cargo test -p <crate> --lib
cargo run -p usql-builder --example builder   # prints generated SQL
cargo test --workspace --no-fail-fast         # needs a C compiler
cargo run -p usql --example project           # needs a C compiler
cargo fmt -p <crate>
```

- Git dependencies (bycat, geob, klaver) need network access on first fetch.
- `usql-quick` and bundled SQLite compile C code, so they need a C toolchain
  and system headers. Dev-dependencies pull in SQLite for `usql` and
  `usql-migrate`, so their tests and examples need it too.
- Workspace builds unify features. Check a crate alone with `-p`, and enable
  its relevant `--features`, after touching feature-gated code.
- There are few tests. For builder changes, add unit tests asserting on the
  generated SQL.

- The Postgres tests in `usql-postgres` and `usql-schema` skip themselves (and pass) unless
  `USQL_POSTGRES_URL` points at a disposable database with PostGIS. The TLS test
  also needs `--features rustls` and `USQL_POSTGRES_CA`, the path of a PEM certificate.
- The MySQL tests in `usql-mysql` skip themselves unless `USQL_MYSQL_URL` points at a
  disposable database, e.g. `docker run --rm -d -p 3306:3306 -e MYSQL_ROOT_PASSWORD=pw
  -e MYSQL_DATABASE=usql mysql:8` with `USQL_MYSQL_URL=mysql://root:pw@localhost:3306/usql`.

## Conventions

- Keep the `no_std` crates `no_std`: use `core` / `alloc`, and gate anything
  needing `std` behind a feature.
- Core async traits return `impl Future<...> + Send` instead of using
  `async_trait`; implementations usually return an `async move` block.
  Preserve the `Send` bounds.
- Builder nodes implement `Expression` or `Statement` by writing into
  `Context`. Use `Context::push` for values and `Context::push_identifier` for
  identifiers; never interpolate user values into SQL. In expressions, a plain
  string is an identifier; use `val(...)` for a value.
- PostgreSQL parameters are converted to the type the server expects, and an
  error if they don't fit (`usql-value/src/postgres`). `NUMERIC` is read as text,
  and `TIMESTAMPTZ` as a naive timestamp in UTC.
- `usql-schema` has one module per database and no changes to the driver traits.
  Schema and table names are bound, or quoted where SQLite requires an identifier.
  What `to_builder` can't express must be added to `omitted`.
- MySQL `DECIMAL` is read as text, and `BINARY(16)` (the builder's `Uuid`) as bytes unless
  read as `Type::Uuid`. The driver checks the parameter count itself, because mysql_async
  closes the connection on client-side errors.
- Branch on `Context::dialect()` where SQL differs between systems.
- The SQLite connection lives on a dedicated thread (`usql-sqlite/src/worker.rs`)
  and is driven by messages. New operations need a `Request` variant, and a
  `TransRequest` variant if they must work inside a transaction.
- Private modules with selective `pub use` in `lib.rs` are the norm.

## Cross-cutting changes

- **Core trait change:** update sqlite, postgres, `usql-any`, the `usql` facade
  and any affected consumers (`usql-migrate`, `usql-quick`).
- **New value variant:** update `Value`, `ValueRef`, `Type`, `convert/`, JSON
  handling and each driver's conversions.
- **New backend:** implement the core traits, then wire it into `usql-any`
  behind a feature.

## Known issues

Remove entries here as they are fixed.

- geob 0.2 stores EWKB and rejects geometry blobs written to SQLite by earlier
  versions; they must be re-encoded.
- The `usql-value` test `ty::tests::test_type_display` fails (expects `real`,
  gets `float`).
- `cargo fmt --all --check` fails on existing code; avoid unrelated
  formatting changes in your diffs.
- MySQL has no `RETURNING` (MariaDB 10.5+ does); the builder still emits it.
- `usql-mysql` can't send `Array` or `Geometry` parameters, and reads geometry as raw bytes.
- `usql-schema` has no MySQL module; `Inspector` returns `Unsupported` on MySQL.
