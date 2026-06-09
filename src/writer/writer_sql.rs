//! SQL query file writer.

use std::{error::Error, path::PathBuf};
use std::io::Write;

use crate::engine::handlebars_model::RenderedModel;

/// Write raw SQL query files under `<project_path>/src/sql/<model>/<method>.sql`.
///
/// Creates the per-model sub-directory if it does not exist.
///
/// # Arguments
///
/// * `project_path` – root of the generated project.
/// * `models` – list of `(method_name, rendered_model)` pairs (e.g. `("get", …)`).
///
/// # Errors
///
/// Returns an error if any directory or file cannot be created, or if writing fails.
pub fn write(
    project_path: &PathBuf,
    models: Vec<(String, RenderedModel)>,
) -> Result<(), Box<dyn Error>> {

    let sql_dir = project_path.join("src/sql");

    for (method, model) in models {
        let dir_name = sql_dir.join(&model.name);
        std::fs::create_dir_all(&dir_name)?;
        let file_path = dir_name.join(format!("{}.sql", method));
        let mut file = std::fs::File::create(file_path)?;
        file.write_all(model.content.as_bytes())?;
    }
    Ok(())
}
