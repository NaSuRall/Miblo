//! High-level code generation service.

use std::error::Error;
use std::path::PathBuf;
use crate::generator::{
    generator_handlers, generator_models, generator_routes,
    generator_sql, generator_sqlx, generator_tempalte,
};
use crate::cli::config::MibloConfig;

/// Orchestrates all code generators for a single project initialisation.
///
/// Holds the resolved project path and the fully-parsed config so individual
/// generators do not need to re-read any files.
pub struct Generator {
    project_path: PathBuf,
    miblo_config: MibloConfig,
}

impl Generator {
    /// Create a new `Generator` bound to `project_path` and `miblo_config`.
    pub fn new(project_path: PathBuf, miblo_config: MibloConfig) -> Self {
        Self { project_path, miblo_config }
    }

    /// Run every generator in order and produce the complete project scaffold.
    ///
    /// Generators run sequentially:
    /// 1. Project template (Cargo.toml, main.rs, …)
    /// 2. SQLx migration
    /// 3. Model structs
    /// 4. SQL query files
    /// 5. Axum router
    /// 6. Axum handlers
    ///
    /// # Errors
    ///
    /// Returns the first error encountered by any generator.
    pub fn generator_all(&self, name: &str) -> Result<(), Box<dyn Error>> {
        generator_tempalte::template(&self.project_path, name, &self.miblo_config)?;
        generator_sqlx::generate(&self.project_path, &self.miblo_config)?;
        generator_models::generate(&self.project_path, &self.miblo_config)?;
        generator_sql::generate(&self.project_path, &self.miblo_config)?;
        generator_routes::generate(&self.project_path, &self.miblo_config)?;
        generator_handlers::generate(&self.project_path, &self.miblo_config)?;
        Ok(())
    }
}
