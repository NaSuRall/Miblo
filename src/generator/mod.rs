pub mod generator_yaml;

use include_dir::{include_dir, Dir};
use std::env;
use std::fs;
use std::io;
use std::path::Path;
// use colored::*;
static TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/templates");

pub fn generator(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Quel langage souhaitez vous pour votre api ?");
    println!("1) RUST");

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice: usize = input.trim().parse().unwrap();

    let template = match choice {
        1 => TEMPLATES.get_dir("rust_api").unwrap(),
        _ => {
            println!("Choix invalide");
            return Ok(());
        }
    };

    let current_dir = env::current_dir()?;
    let project_path = current_dir.join(name);

    fs::create_dir_all(&project_path)?;

    copy_dir(template, &project_path)?;

    // println!("API créé dans {:?}", project_path);

    Ok(())
}

fn copy_dir(dir: &Dir, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for file in dir.files() {
        let path = dest.join(file.path().file_name().unwrap());
        fs::write(path, file.contents())?;
    }

    for subdir in dir.dirs() {
        let new_dest = dest.join(subdir.path().file_name().unwrap());
        fs::create_dir_all(&new_dest)?;
        copy_dir(subdir, &new_dest)?;
    }

    Ok(())
}