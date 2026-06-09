//! Interactive directory-creation helper.

use colored::*;
use console_utils::input::select;
use std::path::PathBuf;

/// Ask the user to confirm project creation, then call [`std::fs::create_dir_all`].
///
/// Displays the target path and prompts with `yes / no`. Returns an error if the
/// user selects `no` or if directory creation fails.
///
/// # Errors
///
/// * `"Your api was not create"` – user declined the prompt.
/// * Any [`std::io::Error`] from [`std::fs::create_dir_all`].
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
