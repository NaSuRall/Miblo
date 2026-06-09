//! SQLx migration file writer.

use crate::engine::handlebars_model::RenderedModel;
use std::{error::Error, fs::OpenOptions, path::PathBuf};
use std::io::Write;

/// Append rendered SQL content from every model to the migration file at `migration_dir`.
///
/// Opens the file in append mode so existing content (e.g. the empty file created by
/// `sqlx migrate add`) is preserved.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or if any write fails.
pub fn write_migration(migration_dir: &PathBuf, data: Vec<RenderedModel>) -> Result<(), Box<dyn Error>> {

    let mut file = OpenOptions::new()
        .append(true)
        .open(migration_dir).expect("Failed to open folder migration");

    for line in data {
        writeln!(file, "{}", line.content )
            .expect("Failed to write data");
    }

    Ok(())
}
