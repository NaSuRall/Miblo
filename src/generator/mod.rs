use fs_extra::dir::{CopyOptions, copy};

pub fn generator(name: String) {
    println!("Genération de l'APi {} en rust", name);
    let _ = copy_folder();
}

fn copy_folder() -> fs_extra::error::Result<()> {
    let mut options = CopyOptions::new();
    options.copy_inside = true;
    options.overwrite = true;

    copy("src/templates/rust_api", "src/", &options)?;
    Ok(())
}
