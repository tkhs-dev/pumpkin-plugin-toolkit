//! Shared, reusable helpers for Pumpkin WASM plugins, built on top of
//! `pumpkin-plugin-api`.
//!
//! This crate is organized by feature area, one top-level module per area
//! (mirroring `pumpkin_plugin_api`'s own `command`/`text`/`world` split).
//! Adding a new area later is just a new `src/<area>.rs` (or `src/<area>/`)
//! plus a `pub mod <area>;` line below — existing modules don't need to
//! change.

/// Command-tree helpers: declarative, composable guards (permission checks,
/// player-only restrictions, ...) applied at the `.execute(...)` call site
/// instead of duplicated inside every handler's `handle()` body.
pub mod command;
