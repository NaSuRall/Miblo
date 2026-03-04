use std::env::current_dir;
use std::process::{Command, Stdio};

pub fn runtime(name: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Serveur lancer ");
    // Lancer le serveur web axum de l'API générer
    let current_dir = current_dir().expect("Impossible de trouver le dossier current");

    let project_path = current_dir.join(&name);

    let mut start = Command::new("cargo")
        .arg("run")
        .current_dir(&project_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let status = start.wait()?;

    if !status.success() {
        return Err("Erreur lors du lancement du serveur".into());
    }

    Ok(())
}
