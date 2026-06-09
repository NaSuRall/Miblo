//! Domain model and field definitions.

use serde::Deserialize;

/// A domain model declared in the `models:` section of `config.yaml`.
///
/// # Example YAML
///
/// ```yaml
/// models:
///   - name: User
///     method: [get, post, patch, delete]
///     fields:
///       - name: id
///         type: Int
///         primary_key: true
///         auto_increment: true
///       - name: email
///         type: String
///         not_null: true
///         unique: true
/// ```
#[derive(Debug, Deserialize)]
pub struct Model {
    /// PascalCase name of the model (e.g. `"User"`). Used as the Rust struct name.
    pub name: String,
    /// HTTP methods to generate for this model (e.g. `["get", "post", "patch", "delete"]`).
    pub method: Vec<String>,
    /// Ordered list of fields that make up the model.
    pub fields: Vec<Field>,
}

/// A single field inside a [`Model`].
#[derive(Debug, Deserialize)]
pub struct Field {
    /// Snake_case field name (e.g. `"created_at"`).
    pub name: String,
    /// Miblo type name (`"String"`, `"Int"`, `"Binary"`). Mapped to Rust/SQL types by the engine.
    // `r#type` is needed because `type` is a reserved keyword in Rust.
    pub r#type: String,
    /// Whether this field is the primary key of the table.
    pub primary_key: Option<bool>,
    /// Whether this field auto-increments (typically only meaningful for integer primary keys).
    pub auto_increment: Option<bool>,
    /// Whether a `NOT NULL` constraint should be added in the SQL migration.
    pub not_null: Option<bool>,
    /// Whether a `UNIQUE` constraint should be added in the SQL migration.
    pub unique: Option<bool>,
}
