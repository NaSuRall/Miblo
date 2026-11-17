use anyhow::{Context, Result, ensure};
use std::{path::Path, process::Command};

pub fn start(project: &Path) -> Result<()> {
    ensure!(
        project.join("Cargo.toml").is_file(),
        "No Cargo.toml in {}",
        project.display()
    );
    let mut command = Command::new("cargo");
    command.arg("run").current_dir(project);
    if project.join("miblo.yaml").is_file() {
        command.env("DATABASE_URL", crate::database::connection_url(project)?);
    }
    let status = command
        .status()
        .context("Cannot run Cargo. Install the Rust toolchain and check your PATH")?;
    ensure!(
        status.success(),
        "The API exited with {status}; see the output above"
    );
    Ok(())
}

pub fn doctor(project: &Path) -> Result<()> {
    let (config, state) = crate::generation::inspect(project)?;
    println!("Project: {} / {}", config.name, config.stack);
    println!("Template: {} (pinned)", state.template.version);
    println!("Resources: {}", config.resources.len());
    let status = Command::new("cargo")
        .arg("--version")
        .status()
        .context("Cargo is not installed")?;
    ensure!(status.success(), "Cargo is not available");
    if crate::database::connection_url(project).is_err() {
        println!("Setup needed: open Miblo and press b to configure the database.");
    }
    println!("Project configuration and generated route registry are consistent.");
    println!(
        "Use miblo db check or the b menu to test connectivity; migrations are applied at startup or with miblo db migrate."
    );
    Ok(())
}
