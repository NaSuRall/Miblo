pub mod health;
pub mod schema;

use crate::config::{Database, Project};
use anyhow::{Context, Result, bail, ensure};
use sqlx::{
    ConnectOptions, Connection, Executor, MySql,
    migrate::{MigrateDatabase, Migrator},
};
use std::{fs, io::Write, path::Path, str::FromStr, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum Action {
    /// Inspect actual tables, columns, primary keys and foreign keys (JSON).
    Schema,
    Check,
    Migrate,
    Init,
}

#[derive(Default)]
pub struct MysqlSettings {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub options: Option<String>,
}

impl MysqlSettings {
    pub fn url(&self) -> Result<String> {
        ensure!(!self.host.is_empty(), "Indique le serveur MySQL");
        ensure!(self.port > 0, "Port MySQL invalide");
        ensure!(
            !self.database.is_empty()
                && self.database.len() <= 64
                && self
                    .database
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_'),
            "Le nom de base doit contenir lettres, chiffres ou underscores (64 maximum)"
        );
        ensure!(!self.username.is_empty(), "Indique l’utilisateur MySQL");
        ensure!(
            !self.username.chars().any(char::is_control)
                && !self.password.chars().any(char::is_control),
            "Les identifiants ne doivent pas contenir de caractères de contrôle"
        );
        let mut url = reqwest::Url::parse("mysql://localhost")?;
        url.set_host(Some(&self.host))
            .map_err(|_| anyhow::anyhow!("Serveur MySQL invalide"))?;
        url.set_port(Some(self.port))
            .map_err(|_| anyhow::anyhow!("Port MySQL invalide"))?;
        url.set_path(&self.database);
        url.set_query(self.options.as_deref());
        let encode = |value: &str| {
            percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC)
                .to_string()
        };
        url.set_username(&encode(&self.username))
            .map_err(|_| anyhow::anyhow!("Utilisateur invalide"))?;
        url.set_password(Some(&encode(&self.password)))
            .map_err(|_| anyhow::anyhow!("Mot de passe invalide"))?;
        Ok(url.into())
    }

    pub fn load(root: &Path) -> Self {
        let fallback = || Self {
            host: "127.0.0.1".into(),
            port: 3306,
            database: "miblo_dev".into(),
            username: "miblo".into(),
            password: String::new(),
            options: None,
        };
        let Ok(url) =
            connection_url(root).and_then(|s| reqwest::Url::parse(&s).map_err(Into::into))
        else {
            return fallback();
        };
        if url.scheme() != "mysql" {
            return fallback();
        }
        let decode = |v: &str| {
            percent_encoding::percent_decode_str(v)
                .decode_utf8_lossy()
                .into_owned()
        };
        Self {
            host: url.host_str().unwrap_or("127.0.0.1").into(),
            port: url.port().unwrap_or(3306),
            database: decode(url.path().trim_start_matches('/')),
            username: decode(url.username()),
            password: decode(url.password().unwrap_or("")),
            options: url.query().map(str::to_owned),
        }
    }
}

fn env_text(root: &Path) -> Result<Option<String>> {
    let path = root.join(".env");
    match path.symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
        Ok(info) => {
            ensure!(
                info.file_type().is_file() && info.len() <= 1024 * 1024,
                ".env doit être un fichier normal de moins de 1 Mio"
            );
            Ok(Some(
                fs::read_to_string(path).context("Impossible de lire .env")?,
            ))
        }
    }
}

pub fn connection_url(root: &Path) -> Result<String> {
    let project = Project::load(&root.join("miblo.yaml"))?;
    if let Some(text) = env_text(root)? {
        for item in dotenvy::from_read_iter(text.as_bytes()) {
            let (key, value) = item.map_err(|_| {
                anyhow::anyhow!("Fichier .env invalide ; corrige sa syntaxe avant de continuer")
            })?;
            if key == "DATABASE_URL" {
                return validate_url(project.database(), value);
            }
        }
    }
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return validate_url(project.database(), url);
    }
    if project.database() == Database::Sqlite {
        return Ok("sqlite://miblo.sqlite?mode=rwc".into());
    }
    bail!("MySQL n’est pas configuré : ouvre Base de données avec b pour saisir la connexion")
}

pub fn env_value(root: &Path, name: &str) -> Result<Option<String>> {
    if let Some(text) = env_text(root)? {
        for item in dotenvy::from_read_iter(text.as_bytes()) {
            let (key, value) = item.map_err(|_| anyhow::anyhow!("Fichier .env invalide"))?;
            if key == name {
                return Ok(Some(value));
            }
        }
    }
    Ok(None)
}

fn validate_url(database: Database, url: String) -> Result<String> {
    ensure!(
        url.starts_with(&format!("{}:", database.arg())),
        "DATABASE_URL ne correspond pas au moteur du projet"
    );
    Ok(url)
}

pub fn target_label(root: &Path) -> Result<String> {
    let project = Project::load(&root.join("miblo.yaml"))?;
    let url = connection_url(root)?;
    match project.database() {
        Database::Sqlite => {
            let options = sqlx::sqlite::SqliteConnectOptions::from_str(&url)
                .map_err(|_| anyhow::anyhow!("URL SQLite invalide"))?;
            Ok(format!("SQLite : {}", options.get_filename().display()))
        }
        Database::Mysql => {
            let options = sqlx::mysql::MySqlConnectOptions::from_str(&url)
                .map_err(|_| anyhow::anyhow!("URL MySQL invalide"))?;
            Ok(format!(
                "MySQL : {}:{} / {}",
                options.get_host(),
                options.get_port(),
                options.get_database().unwrap_or("")
            ))
        }
    }
}

pub fn save_mysql(root: &Path, settings: &MysqlSettings) -> Result<()> {
    let project = Project::load(&root.join("miblo.yaml"))?;
    ensure!(
        project.database() == Database::Mysql,
        "Ce projet utilise SQLite"
    );
    let url = settings.url()?;
    let text = env_text(root)?
        .unwrap_or_else(|| format!("SERVER_ADDRESS=127.0.0.1\nSERVER_PORT={}\n", project.port));
    // Validate first, then preserve unrelated entries and comments. Single quotes
    // prevent dotenv interpolation; URL credentials are percent-encoded.
    for item in dotenvy::from_read_iter(text.as_bytes()) {
        item.map_err(|_| anyhow::anyhow!("Fichier .env invalide ; aucune modification effectuée"))?;
    }
    let replacement = format!("DATABASE_URL='{url}'\n");
    let mut other = String::new();
    let mut record = String::new();
    let mut replaced = false;
    // Parse complete dotenv records, including quoted multiline values. Keep
    // unrelated bytes and the original key order (interpolation is order-sensitive).
    for line in text.split_inclusive('\n') {
        record.push_str(line);
        let Ok(entries) =
            dotenvy::from_read_iter(record.as_bytes()).collect::<std::result::Result<Vec<_>, _>>()
        else {
            continue;
        };
        if entries.iter().any(|(key, _)| key == "DATABASE_URL") {
            if !replaced {
                other.push_str(&replacement);
                replaced = true;
            }
        } else {
            other.push_str(&record);
        }
        record.clear();
    }
    ensure!(
        record.is_empty(),
        "Fichier .env invalide ; aucune modification effectuée"
    );
    if !replaced {
        if !other.is_empty() && !other.ends_with('\n') {
            other.push('\n');
        }
        other.push_str(&replacement);
    }
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(other.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(root.join(".env")).map_err(|e| e.error)?;
    Ok(())
}

pub fn run(root: &Path, action: Action) -> Result<()> {
    if action == Action::Schema {
        println!("{}", serde_json::to_string_pretty(&schema::inspect(root)?)?);
        return Ok(());
    }
    let root = root.canonicalize()?;
    let project = Project::load(&root.join("miblo.yaml"))?;
    let url = connection_url(&root)?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let task = async {
            match project.database() {
                Database::Sqlite => {
                    let mut options = sqlx::sqlite::SqliteConnectOptions::from_str(&url)
                        .map_err(|_| anyhow::anyhow!("URL SQLite invalide"))?;
                    let filename = options.get_filename().to_path_buf();
                    ensure!(filename != Path::new(":memory:"), "Utilise un fichier SQLite persistant pour gérer les migrations");
                    let filename = if filename.is_absolute() { filename } else { root.join(filename) };
                    ensure!(!filename.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()), "Le fichier SQLite ne doit pas être un lien symbolique");
                    options = options.filename(filename).create_if_missing(action != Action::Check)
                        .foreign_keys(true).busy_timeout(Duration::from_secs(10)).disable_statement_logging();
                    let mut connection = sqlx::SqliteConnection::connect_with(&options).await
                        .map_err(|_| anyhow::anyhow!("Connexion SQLite impossible ; initialise la base avec b → Initialiser"))?;
                    if action == Action::Check { connection.execute("SELECT 1").await?; }
                    else { Migrator::new(root.join("migrations")).await?.run(&mut connection).await?; }
                    connection.close().await?;
                }
                Database::Mysql => {
                    let options = sqlx::mysql::MySqlConnectOptions::from_str(&url)
                        .map_err(|_| anyhow::anyhow!("URL MySQL invalide"))?.disable_statement_logging();
                    if action == Action::Init && !MySql::database_exists(&url).await.map_err(|_| anyhow::anyhow!("Impossible de joindre MySQL ; vérifie le serveur et les identifiants"))? {
                        MySql::create_database(&url).await.map_err(|_| anyhow::anyhow!("Création de la base refusée ; vérifie les droits de l’utilisateur MySQL"))?;
                    }
                    let mut connection = sqlx::MySqlConnection::connect_with(&options).await
                        .map_err(|_| anyhow::anyhow!("Connexion MySQL impossible ; vérifie le serveur, la base et les identifiants avec b"))?;
                    if action == Action::Check { connection.execute("SELECT 1").await?; }
                    else { Migrator::new(root.join("migrations")).await?.run(&mut connection).await?; }
                    connection.close().await?;
                }
            }
            Ok::<(), anyhow::Error>(())
        };
        tokio::time::timeout(Duration::from_secs(30), task).await
            .context("La base n’a pas répondu dans le délai de 30 secondes")??;
        Ok::<(), anyhow::Error>(())
    })?;
    println!(
        "{} : {}",
        project.database().name(),
        if action == Action::Check {
            "connexion vérifiée"
        } else {
            "base prête, migrations appliquées"
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mysql_credentials_roundtrip_and_env_preserves_other_values() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::new("sample".into(), 4000, vec![]);
        fs::write(
            dir.path().join("miblo.yaml"),
            serde_yaml::to_string(&project).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.path().join(".env"),
            "# custom\nFEATURE=keep\nDATABASE_URL=mysql://old:old@localhost/old\n",
        )
        .unwrap();
        let settings = MysqlSettings {
            host: "localhost".into(),
            port: 3307,
            database: "test_db".into(),
            username: "u@ser".into(),
            password: "p$'\\#:@% secret".into(),
            options: Some("ssl-mode=required".into()),
        };
        save_mysql(dir.path(), &settings).unwrap();
        let loaded = MysqlSettings::load(dir.path());
        assert_eq!(loaded.password, settings.password);
        assert_eq!(loaded.username, settings.username);
        assert_eq!(loaded.port, 3307);
        assert_eq!(loaded.options, settings.options);
        assert!(!target_label(dir.path()).unwrap().contains("secret"));
        assert!(
            fs::read_to_string(dir.path().join(".env"))
                .unwrap()
                .starts_with("# custom\nFEATURE=keep\n")
        );
        let multiline = "NOTES='first\nDATABASE_URL=part-of-a-note\nlast'\nDATABASE_URL=mysql://old@localhost/old\nCOPY=${DATABASE_URL}\n";
        fs::write(dir.path().join(".env"), multiline).unwrap();
        save_mysql(dir.path(), &settings).unwrap();
        let saved = fs::read_to_string(dir.path().join(".env")).unwrap();
        assert!(saved.starts_with("NOTES='first\nDATABASE_URL=part-of-a-note\nlast'\n"));
        let values = dotenvy::from_read_iter(saved.as_bytes())
            .collect::<std::result::Result<std::collections::HashMap<_, _>, _>>()
            .unwrap();
        assert_eq!(values["COPY"], values["DATABASE_URL"]);
    }
}
