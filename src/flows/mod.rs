//! Crate-private flow families. A family exists only when two assets share
//! it, and it is written over the kits. A family never imports an asset
//! module; `tests/dependency_rule.rs` enforces that.
