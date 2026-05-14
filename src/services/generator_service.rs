use std::error::Error;
use std::path::PathBuf;
use crate::generator::{
    generator_handlers, generator_models, generator_routes,
    generator_sql, generator_sqlx, generator_tempalte,
};
use crate::cli::config::MibloConfig;


pub struct Generator{
    project_path: PathBuf,
    miblo_config: MibloConfig
}
impl Generator {

    pub fn new(project_path: PathBuf, miblo_config: MibloConfig) -> Self {
        Self { project_path, miblo_config }
    }

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
