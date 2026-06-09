//! Runtime launcher for the generated project.
//!
//! Runs `sqlx migrate run` (best-effort) and then `cargo run` inside the project
//! directory, streaming stdout through a spinner until the server reports it is ready.

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::env::current_dir;
use std::io::BufRead;
use std::io::BufReader;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Start the generated project identified by `name`.
///
/// 1. Resolves `<current_dir>/<name>` as the project directory.
/// 2. Runs `sqlx migrate run` (failures are silently ignored).
/// 3. Spawns `cargo run` and streams its stdout line by line.
/// 4. Stops the spinner and prints a success message when a line containing
///    `"Server running on"` is detected.
///
/// # Errors
///
/// Returns an error if `cargo run` cannot be spawned or if the child process exits
/// with a non-zero status.
pub fn start(name: &String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );

    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner.set_message(" Compilation des dépendances...");

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
