use colored::*;
use console_utils::input::select;
use std::path::PathBuf;

pub fn request_create_folder(project_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // demander cli a crée le projet

    let options = ["yes", "no"];

    let message = format!("Create your api here ? {}", project_path.display())
        .cyan()
        .bold();

    let selected_option = select(&message, &options);

    if options[selected_option] != "yes" {
        return Err("Your api was not create".into());
    }
    std::fs::create_dir_all(project_path)?;

    Ok(())
}
