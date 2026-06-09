//! HTTP route configuration model.

use serde::Deserialize;

/// An HTTP route declared in the `routes:` section of `config.yaml`.
///
/// # Example YAML
///
/// ```yaml
/// routes:
///   - method: get
///     model: User
///     path: /users
/// ```
#[derive(Debug, Deserialize, Clone)]
pub struct Route {
    /// HTTP method in lowercase (e.g. `"get"`, `"post"`, `"patch"`, `"delete"`).
    pub method: String,
    /// Name of the [`crate::models::models_models::Model`] this route targets.
    pub model: String,
    /// URL path (e.g. `"/users"` or `"/users/:id"`).
    pub path: String,
}
