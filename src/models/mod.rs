//! Deserialization structs that mirror the `config.yaml` schema.
//!
//! All structs derive [`serde::Deserialize`] so they can be hydrated directly
//! from the parsed YAML value tree.

pub mod models_route;
pub mod models_models;
pub mod models_database;
pub mod models_server;
