use fs_extra::dir::{CopyOptions, copy as copy_dir};
use fs_extra::file::copy as copy_file;
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn generator(name: String) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let template_path = PathBuf::from("src/templates/rust_api");
    let project_path = current_dir.join(&name);

    fs::create_dir_all(&project_path)?;

    let mut dir_options = CopyOptions::new();
    dir_options.overwrite = true;

    for entry in fs::read_dir(&template_path)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = project_path.join(entry.file_name());

        if path.is_dir() {
            copy_dir(&path, &project_path, &dir_options)?;
        } else {
            copy_file(&path, &dest_path, &fs_extra::file::CopyOptions::new())?;
        }
    }

    println!("Ajout de l'api : {} : OK", name);

    Ok(())
}
