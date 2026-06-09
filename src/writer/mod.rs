//! File-system writers for every generated artefact.
//!
//! Each sub-module writes one category of output files:
//!
//! | Module | Writes |
//! |--------|--------|
//! | [`writer_handlers`] | `src/handlers/<model>.rs` + `src/handlers/mod.rs` |
//! | [`writer_migration`] | Appends to the SQLx migration file created by `sqlx migrate add` |
//! | [`writer_models`] | `src/models/<model>.rs` + `src/models/mod.rs` |
//! | [`writer_routes`] | `src/routes/mod.rs` |
//! | [`writer_sql`] | `src/sql/<model>/{get,post,patch,delete}.sql` |

pub mod writer_handlers;
pub mod writer_migration;
pub mod writer_models;
pub mod writer_routes;
pub mod writer_sql;
