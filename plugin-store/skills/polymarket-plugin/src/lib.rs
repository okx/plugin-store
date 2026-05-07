// Library entry point — exposes internal modules for integration tests.
// The binary (main.rs) has its own module declarations pointing to the same files;
// this lib target exists solely so that `tests/` can import crate internals.

pub mod api;
pub mod auth;
pub mod commands;
pub mod config;
pub mod onchainos;
pub mod sanitize;
pub mod series;
pub mod signing;
