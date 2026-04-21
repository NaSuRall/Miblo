use std::env::current_dir;
use std::process::{Command, Stdio};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use console::style;
use std::io::BufRead;
use std::io::BufReader;

pub fn runtime(name: String) -> Result<(), Box<dyn std::error::Error>> {


    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .template("{spinner:.cyan} {msg}")
        .unwrap(),
    );

    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner.set_message("Compilation des dépendances...");


    // Lancer le serveur web axum de l'API générer
    let current_dir = current_dir().expect("Impossible de trouver le dossier current");

    let project_path = current_dir.join(&name);

    let _command = Command::new("sqlx")
        .args(["migrate", "run"])
        .current_dir(&project_path)
        .output();

    let mut start = Command::new("cargo")
        .arg("run")
        .current_dir(&project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;


    let stdout = start.stdout.take().unwrap();
        for line in BufReader::new(stdout).lines() {
            let line = line?;
                if line.contains("Server running on") {
                    spinner.finish_and_clear();
                    println!("{} {}", style("✓").green().bold(), style(&line).cyan());
                break;
        }
    }

    start.wait()?;
    Ok(())
}
