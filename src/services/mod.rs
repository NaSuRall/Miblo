//! Orchestration layer that wires parsers, generators, and writers together.
//!
//! [`generator_service`] is the main entry-point; `config_service` and `writer_service`
//! are reserved for future use.

pub mod config_service;
pub mod generator_service;
pub mod writer_service;
