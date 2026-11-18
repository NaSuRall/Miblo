use anyhow::{Context, Result, ensure};
use std::{path::Path, process::Command};

pub fn http_base(project: &Path) -> Result<String> {
    let config = crate::config::Project::load(&project.join("miblo.yaml"))?;
    // Match dotenvy at API startup: inherited server settings take precedence.
    let value = |key: &str| -> Result<Option<String>> {
        Ok(std::env::var(key)
            .ok()
            .or(crate::database::env_value(project, key)?))
    };
    let port = value("SERVER_PORT")?
        .unwrap_or_else(|| config.port.to_string())
        .parse::<u16>()
        .context("SERVER_PORT invalide")?;
    ensure!(
        port > 0,
        "Indique un SERVER_PORT fixe pour tester les routes"
    );
    let host = value("SERVER_ADDRESS")?.unwrap_or_else(|| "127.0.0.1".into());
    let host = match host.as_str() {
        "0.0.0.0" => "127.0.0.1",
        "::" | "[::]" => "[::1]",
        host => host,
    };
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.into()
    };
    let url = reqwest::Url::parse(&format!("http://{host}:{port}/"))
        .context("SERVER_ADDRESS invalide")?;
    ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        trf!(
            "SERVER_ADDRESS must be a hostname or IP address",
            "SERVER_ADDRESS doit être un nom d’hôte ou une adresse IP"
        )
    );
    Ok(url.to_string().trim_end_matches('/').to_owned())
}

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
