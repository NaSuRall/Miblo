//! YAML parsing and configuration hydration.
//!
//! * [`config_reader`] – opens and parses the raw `config.yaml` into a [`serde_yaml::Value`].
//! * [`reader_yaml`] – deserializes that value into a typed [`crate::cli::config::MibloConfig`].

pub mod config_reader;
pub mod reader_yaml;
