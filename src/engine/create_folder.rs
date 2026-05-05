use std::path::PathBuf;
use colored::*;
use console_utils::input::select;

pub fn request_create_folder(project_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // demander cli a crée le projet
    let options = [
        "yes", "no"
    ];
    let selected_option =  select("Create your api here ?", &options);
    println!("Selected : {}", options[selected_option]);

    if  options[selected_option] != "yes"{
        return Err("Your api was not create".into());
    }
    std::fs::create_dir_all(project_path)?;

    Ok(())
}
