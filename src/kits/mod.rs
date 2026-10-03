//! Crate-private building blocks. Each kit does one job and depends only on
//! kits below it, `canton-lib` and third-party crates. A kit never imports a
//! flow family or an asset module; `tests/dependency_rule.rs` enforces that.
