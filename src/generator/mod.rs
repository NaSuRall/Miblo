//! Per-artefact code generators.
//!
//! Each sub-module is responsible for one type of output file:
//!
//! | Module | Generated artefact |
//! |--------|--------------------|
//! | [`generator_handlers`] | Axum handler functions (`src/handlers/<model>.rs`) |
//! | [`generator_models`] | SQLx model structs (`src/models/<model>.rs`) |
//! | [`generator_routes`] | Axum router (`src/routes/mod.rs`) |
//! | [`generator_server`] | Server entry-point boilerplate |
//! | [`generator_sql`] | Raw SQL query files (`src/sql/<model>/{get,post,patch,delete}.sql`) |
//! | [`generator_sqlx`] | SQLx migration file |
//! | [`generator_tempalte`] | Project scaffold (`Cargo.toml`, `main.rs`, `.env`, …) |

pub mod generator_handlers;
pub mod generator_models;
pub mod generator_routes;
pub mod generator_server;
pub mod generator_sql;
pub mod generator_sqlx;
pub mod generator_tempalte;
