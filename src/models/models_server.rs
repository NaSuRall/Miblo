//! Server binding configuration model.

use serde::Deserialize;

/// Server binding settings read from the `server:` section of `config.yaml`.
///
/// Both fields are optional; generated code falls back to `0.0.0.0:3000` when omitted.
///
/// # Example YAML
///
/// ```yaml
/// server:
///   - port: 8080
///     address: "127.0.0.1"
/// ```
#[derive(Debug, Deserialize)]
pub struct Server {
    /// TCP port the generated API will listen on (defaults to `3000` if absent).
    pub port: Option<u16>,
    /// Bind address for the generated API (defaults to `"0.0.0.0"` if absent).
    pub address: Option<String>,
}
