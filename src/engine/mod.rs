//! Core engine utilities shared by all generators.
//!
//! * [`create_folder`] – interactive prompt before creating the project directory.
//! * [`handlebars_model`] – renders Handlebars templates for every model in the config.
//! * [`type_rust`] – maps Miblo type names to Rust and SQL type strings.

pub mod create_folder;
pub mod handlebars_model;
pub mod type_rust;
