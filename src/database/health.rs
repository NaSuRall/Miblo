use super::*;
use sqlx::Row;

#[derive(Clone, Debug, PartialEq)]
pub enum Health {
    Unknown,
    Ready,
    Pending(usize),
    Initialize,
    Unavailable(String),
    Drift(String),
}

type Applied = Vec<(i64, Vec<u8>, bool)>;

fn compare(migrator: &Migrator, applied: Applied, initialized: bool) -> Health {
    if !initialized {
        return Health::Initialize;
    }
    for (version, checksum, success) in &applied {
        if !success {
            return Health::Drift(trf!(
                "Migration {version} is incomplete",
                "Migration {version} incomplète"
            ));
        }
        let Some(source) = migrator
            .iter()
            .find(|m| m.version == *version && m.migration_type.is_up_migration())
        else {
            return Health::Drift(trf!(
                "Migration {version} is missing from the project",
                "Migration {version} absente du projet"
            ));
        };
        if source.checksum.as_ref() != checksum {
            return Health::Drift(trf!(
                "Migration {version} was modified after being applied",
                "Migration {version} modifiée après application"
            ));
        }
    }
    let pending = migrator
        .iter()
        .filter(|m| {
            m.migration_type.is_up_migration()
                && !applied.iter().any(|(version, _, _)| *version == m.version)
        })
        .count();
    if pending > 0 {
        Health::Pending(pending)
    } else {
        Health::Ready
    }
}

pub fn inspect(root: &Path) -> Health {
    let result = (|| -> Result<Health> {
        let config = Project::load(&root.join("miblo.yaml"))?;
        let url = connection_url(root)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let task = async {
                let migrator = Migrator::new(root.join("migrations")).await
                    .map_err(|_| anyhow::anyhow!(trf!("Invalid migration files", "Fichiers de migration invalides")))?;
                match config.database() {
                    Database::Sqlite => {
                        let mut options = sqlx::sqlite::SqliteConnectOptions::from_str(&url)
                            .map_err(|_| anyhow::anyhow!(trf!("Invalid SQLite configuration", "Configuration SQLite invalide")))?;
                        let file = options.get_filename().to_path_buf();
                        let file = if file.is_absolute() { file } else { root.join(file) };
                        if !file.exists() { return Ok(Health::Initialize); }
                        ensure!(!file.symlink_metadata()?.file_type().is_symlink(), trf!("SQLite symbolic links are not allowed", "Lien symbolique SQLite refusé"));
                        // Status checks never create a database or migration metadata.
                        options = options.filename(file).create_if_missing(false).read_only(true)
                            .busy_timeout(Duration::from_millis(500)).disable_statement_logging();
                        let mut connection = sqlx::SqliteConnection::connect_with(&options).await
                            .map_err(|_| anyhow::anyhow!(trf!("Cannot connect to SQLite", "Connexion SQLite impossible")))?;
                        let initialized: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'")
                            .fetch_one(&mut connection).await?;
                        let applied = if initialized > 0 {
                            sqlx::query("SELECT version, checksum, success FROM _sqlx_migrations").fetch_all(&mut connection).await?
                                .into_iter().map(|r| Ok((r.try_get(0)?, r.try_get(1)?, r.try_get(2)?)))
                                .collect::<std::result::Result<Applied, sqlx::Error>>()?
                        } else { vec![] };
                        connection.close().await?;
                        Ok(compare(&migrator, applied, initialized > 0))
                    }
                    Database::Mysql => {
                        let options = sqlx::mysql::MySqlConnectOptions::from_str(&url)
                            .map_err(|_| anyhow::anyhow!(trf!("Invalid MySQL configuration", "Configuration MySQL invalide")))?.disable_statement_logging();
                        let mut connection = match sqlx::MySqlConnection::connect_with(&options).await {
                            Ok(connection) => connection,
                            Err(sqlx::Error::Database(error)) if error.try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>()
                                .is_some_and(|error| error.number() == 1049) => return Ok(Health::Initialize),
                            Err(_) => bail!(trf!("Cannot connect to MySQL", "Connexion MySQL impossible")),
                        };
                        let initialized: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema=DATABASE() AND table_name='_sqlx_migrations'")
                            .fetch_one(&mut connection).await?;
                        let applied = if initialized > 0 {
                            sqlx::query("SELECT version, checksum, success FROM _sqlx_migrations").fetch_all(&mut connection).await?
                                .into_iter().map(|r| Ok((r.try_get(0)?, r.try_get(1)?, r.try_get(2)?)))
                                .collect::<std::result::Result<Applied, sqlx::Error>>()?
                        } else { vec![] };
                        connection.close().await?;
                        Ok(compare(&migrator, applied, initialized > 0))
                    }
                }
            };
            tokio::time::timeout(Duration::from_secs(2), task).await
                .context(tr!("Database unreachable (timed out)", "Base injoignable (délai dépassé)"))?
        })
    })();
    match result {
        Ok(health) => health,
        Err(error) => Health::Unavailable(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_real_pending_changed_and_missing_migrations_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Project::new("status-test".into(), 3000, vec![]);
        config.stack = Database::Sqlite.stack().into();
        fs::write(
            dir.path().join("miblo.yaml"),
            serde_yaml::to_string(&config).unwrap(),
        )
        .unwrap();
        fs::create_dir(dir.path().join("migrations")).unwrap();
        let first = dir.path().join("migrations/000001_users.sql");
        fs::write(&first, "CREATE TABLE users (id INTEGER PRIMARY KEY);").unwrap();
        assert_eq!(inspect(dir.path()), Health::Initialize);
        assert!(!dir.path().join("miblo.sqlite").exists());
        run(dir.path(), Action::Init).unwrap();
        assert_eq!(inspect(dir.path()), Health::Ready);
        fs::write(
            dir.path().join("migrations/000002_notes.sql"),
            "CREATE TABLE notes (id INTEGER);",
        )
        .unwrap();
        assert_eq!(inspect(dir.path()), Health::Pending(1));
        run(dir.path(), Action::Migrate).unwrap();
        assert_eq!(inspect(dir.path()), Health::Ready);
        fs::write(
            &first,
            "CREATE TABLE users (id INTEGER PRIMARY KEY, changed TEXT);",
        )
        .unwrap();
        assert!(matches!(inspect(dir.path()), Health::Drift(_)));
        fs::remove_file(&first).unwrap();
        assert!(matches!(inspect(dir.path()), Health::Drift(_)));
    }
}
