use fs_extra::dir::{CopyOptions, copy};
use std::env;

pub fn generator(name: String) -> fs_extra::error::Result<()> {
    let current_dir = env::current_dir().expect("Impossible de récupérer le dossier courant");
    println!("Récupereation du dossier courrant : OK");

    let project_path = current_dir.join(&name);
    std::fs::create_dir_all(&project_path).expect("Impossible de créer le dossier du projet");
    println!("Création du dossier : {} : OK", name);

    let mut options = CopyOptions::new();
    options.copy_inside = true;
    options.overwrite = true;

    copy("src/templates/rust_api", &project_path, &options).expect("Ipossible de crée le dossier");
    println!("Ajout de l'api : {} : OK", name);

    Ok(())
}
