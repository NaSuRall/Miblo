use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::env::current_dir;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

pub fn runtime(name: String) -> Result<(), Box<dyn std::error::Error>> {
    let spinner = Arc::new(ProgressBar::new_spinner());
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner.set_message("Compilation des dépendances...");

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
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = start.stdout.take().unwrap();
    let stderr = start.stderr.take().unwrap();

    let spinner_stderr = Arc::clone(&spinner);
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else { break };
            if line.contains("Compiling") {
                spinner_stderr.set_message(format!("{}", style(line.trim()).dim()));
            }
        }
    });

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
