use crate::engine::handlebars_model::RenderedModel;
use std::{error::Error, fs::OpenOptions, path::PathBuf};
use std::io::Write;

pub fn write_migration( migration_dir: &PathBuf, data: Vec<RenderedModel>) -> Result<(), Box<dyn Error>> {

    let mut file = OpenOptions::new()
        .append(true)
        .open(migration_dir).expect("Failed to open folder migration");

    for line in data {
        writeln!(file, "{}", line.content )
            .expect("Failed to write data");
    }

    Ok(())
}
