//! Database connection configuration model.

use serde::Deserialize;

/// Database connection settings read from the `database:` section of `config.yaml`.
///
/// # Example YAML
///
/// ```yaml
/// database:
///   - db_connexion: postgres
///     db_host: localhost
///     db_port: 5432
///     db_database: my_db
///     db_username: user
///     db_password: secret
/// ```
#[derive(Debug, Deserialize)]
pub struct Database {
    /// Database driver/connection type (e.g. `"postgres"`, `"sqlite"`).
    pub db_connexion: String,
    /// Hostname or IP of the database server.
    pub db_host: String,
    /// TCP port the database server listens on.
    pub db_port: u16,
    /// Name of the database to connect to.
    pub db_database: String,
    /// Login username.
    pub db_username: String,
    /// Login password.
    pub db_password: String,
}
