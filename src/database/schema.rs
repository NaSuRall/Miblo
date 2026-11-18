//! Read-only schema inspection. No migrations or user-row queries run here.
use super::*;
use serde::Serialize;
use sqlx::Row;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct Column {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub primary_key: i64,
    pub default: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub target: String,
    pub target_columns: Vec<String>,
    pub on_delete: String,
    pub on_update: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Schema {
    pub database: String,
    pub tables: Vec<Table>,
}

pub fn declared(project: &Project) -> Vec<Table> {
    project
        .resources
        .iter()
        .map(|resource| {
            let mut columns = vec![Column {
                name: "id".into(),
                sql_type: if project.database() == Database::Sqlite {
                    "INTEGER"
                } else {
                    "BIGINT"
                }
                .into(),
                nullable: false,
                primary_key: 1,
                default: None,
            }];
            columns.extend(resource.fields.iter().map(|field| Column {
                name: field.name.clone(),
                sql_type: field.kind.sql(project.database()).into(),
                nullable: field.nullable,
                primary_key: 0,
                default: None,
            }));
            let foreign_keys = resource
                .fields
                .iter()
                .filter_map(|field| {
                    field.references.as_ref().map(|reference| ForeignKey {
                        columns: vec![field.name.clone()],
                        target: crate::config::Resource::new(reference.resource.clone(), vec![])
                            .table(),
                        target_columns: vec!["id".into()],
                        on_delete: reference.on_delete.sql().into(),
                        on_update: "RESTRICT".into(),
                    })
                })
                .collect();
            Table {
                name: resource.table(),
                columns,
                foreign_keys,
            }
        })
        .collect()
}

pub fn inspect(root: &Path) -> Result<Schema> {
    let config = Project::load(&root.join("miblo.yaml"))?;
    let url = connection_url(root)?;
    let database = target_label(root)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let tables = runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), async {
            match config.database() {
                Database::Sqlite => sqlite(root, &url).await,
                Database::Mysql => mysql(&url).await,
            }
        })
        .await
        .context(tr!(
            "Schema read timed out (5 s)",
            "Lecture du schéma : délai dépassé (5 s)"
        ))?
    })?;
    Ok(Schema { database, tables })
}

async fn sqlite(root: &Path, url: &str) -> Result<Vec<Table>> {
    let mut options = sqlx::sqlite::SqliteConnectOptions::from_str(url).map_err(|_| {
        anyhow::anyhow!(trf!(
            "Invalid SQLite configuration",
            "Configuration SQLite invalide"
        ))
    })?;
    let file = options.get_filename().to_path_buf();
    ensure!(
        file != Path::new(":memory:"),
        trf!(
            "The schema requires a persistent SQLite file",
            "Le schéma nécessite un fichier SQLite persistant"
        )
    );
    let file = if file.is_absolute() {
        file
    } else {
        root.join(file)
    };
    ensure!(file.is_file(), "Base SQLite absente : initialise-la avec b");
    ensure!(
        !file.symlink_metadata()?.file_type().is_symlink(),
        trf!(
            "SQLite symbolic links are not allowed",
            "Lien symbolique SQLite refusé"
        )
    );
    options = options
        .filename(file)
        .create_if_missing(false)
        .read_only(true)
        .busy_timeout(Duration::from_millis(500))
        .disable_statement_logging();
    let mut connection = sqlx::SqliteConnection::connect_with(&options)
        .await
        .map_err(|_| {
            anyhow::anyhow!(trf!(
                "Cannot connect to SQLite",
                "Connexion SQLite impossible"
            ))
        })?;
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name NOT GLOB 'sqlite_*' AND name != '_sqlx_migrations' ORDER BY name LIMIT 501")
        .fetch_all(&mut connection).await?;
    ensure!(
        names.len() <= 500,
        trf!(
            "The schema exceeds 500 tables",
            "Le schéma dépasse 500 tables"
        )
    );
    let mut tables = Vec::new();
    for name in names {
        let rows = sqlx::query("SELECT name, type, \"notnull\", pk, dflt_value FROM pragma_table_xinfo(?) ORDER BY cid")
            .bind(&name).fetch_all(&mut connection).await?;
        let mut columns = Vec::new();
        for row in rows {
            let pk: i64 = row.try_get("pk")?;
            columns.push(Column {
                name: row.try_get("name")?,
                sql_type: row.try_get("type")?,
                nullable: !row.try_get::<bool, _>("notnull")?,
                primary_key: pk,
                default: row.try_get("dflt_value")?,
            });
        }
        // A rowid alias has no separate primary-key index. INTEGER PRIMARY KEY
        // DESC and other index-based primary keys may still allow NULL in SQLite.
        let indexed_pk: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pragma_index_list(?) WHERE origin='pk'")
                .bind(&name)
                .fetch_one(&mut connection)
                .await?;
        if indexed_pk == 0 {
            for column in &mut columns {
                if column.primary_key > 0 && column.sql_type.eq_ignore_ascii_case("INTEGER") {
                    column.nullable = false;
                }
            }
        }
        let rows = sqlx::query("SELECT id, seq, \"table\", \"from\", \"to\", on_delete, on_update FROM pragma_foreign_key_list(?) ORDER BY id, seq")
            .bind(&name).fetch_all(&mut connection).await?;
        let mut keys = BTreeMap::<i64, ForeignKey>::new();
        for row in rows {
            let key = keys.entry(row.try_get("id")?).or_insert(ForeignKey {
                columns: vec![],
                target: row.try_get("table")?,
                target_columns: vec![],
                on_delete: row.try_get("on_delete")?,
                on_update: row.try_get("on_update")?,
            });
            key.columns.push(row.try_get("from")?);
            key.target_columns.push(
                row.try_get::<Option<String>, _>("to")?
                    .unwrap_or_else(|| "<PK>".into()),
            );
        }
        tables.push(Table {
            name,
            columns,
            foreign_keys: keys.into_values().collect(),
        });
    }
    connection.close().await?;
    Ok(tables)
}

async fn mysql(url: &str) -> Result<Vec<Table>> {
    let options = sqlx::mysql::MySqlConnectOptions::from_str(url)
        .map_err(|_| {
            anyhow::anyhow!(trf!(
                "Invalid MySQL configuration",
                "Configuration MySQL invalide"
            ))
        })?
        .disable_statement_logging();
    let mut connection = sqlx::MySqlConnection::connect_with(&options)
        .await
        .map_err(|_| {
            anyhow::anyhow!(trf!(
                "Cannot connect to MySQL; check the database with b",
                "Connexion MySQL impossible ; vérifie la base avec b"
            ))
        })?;
    // MySQL 8.4 can expose metadata names as VARBINARY; request text explicitly,
    // just as for the column and foreign-key metadata below.
    let names: Vec<String> = sqlx::query_scalar("SELECT CAST(TABLE_NAME AS CHAR) FROM information_schema.TABLES WHERE TABLE_SCHEMA=DATABASE() AND TABLE_TYPE='BASE TABLE' AND TABLE_NAME != '_sqlx_migrations' ORDER BY TABLE_NAME LIMIT 501")
        .fetch_all(&mut connection).await?;
    ensure!(
        names.len() <= 500,
        trf!(
            "The schema exceeds 500 tables",
            "Le schéma dépasse 500 tables"
        )
    );
    let mut tables = Vec::new();
    for name in names {
        let rows = sqlx::query("SELECT CAST(c.COLUMN_NAME AS CHAR) AS COLUMN_NAME, CAST(c.COLUMN_TYPE AS CHAR) AS COLUMN_TYPE, CAST(c.IS_NULLABLE AS CHAR) AS IS_NULLABLE, CAST(c.COLUMN_DEFAULT AS CHAR) AS COLUMN_DEFAULT, CAST(COALESCE(k.ORDINAL_POSITION, 0) AS SIGNED) AS pk FROM information_schema.COLUMNS c LEFT JOIN information_schema.KEY_COLUMN_USAGE k ON k.TABLE_SCHEMA=c.TABLE_SCHEMA AND k.TABLE_NAME=c.TABLE_NAME AND k.COLUMN_NAME=c.COLUMN_NAME AND k.CONSTRAINT_NAME='PRIMARY' WHERE c.TABLE_SCHEMA=DATABASE() AND c.TABLE_NAME=? ORDER BY c.ORDINAL_POSITION")
            .bind(&name).fetch_all(&mut connection).await?;
        let mut columns = Vec::new();
        for row in rows {
            columns.push(Column {
                name: row.try_get("COLUMN_NAME")?,
                sql_type: row.try_get("COLUMN_TYPE")?,
                nullable: row.try_get::<String, _>("IS_NULLABLE")? == "YES",
                primary_key: row.try_get("pk")?,
                default: row.try_get("COLUMN_DEFAULT")?,
            });
        }
        let rows = sqlx::query("SELECT CAST(k.CONSTRAINT_NAME AS CHAR) AS CONSTRAINT_NAME, CAST(k.COLUMN_NAME AS CHAR) AS COLUMN_NAME, CAST(CASE WHEN k.REFERENCED_TABLE_SCHEMA=DATABASE() THEN k.REFERENCED_TABLE_NAME ELSE CONCAT(k.REFERENCED_TABLE_SCHEMA, '.', k.REFERENCED_TABLE_NAME) END AS CHAR) AS REFERENCED_TABLE_NAME, CAST(k.REFERENCED_COLUMN_NAME AS CHAR) AS REFERENCED_COLUMN_NAME, CAST(r.DELETE_RULE AS CHAR) AS DELETE_RULE, CAST(r.UPDATE_RULE AS CHAR) AS UPDATE_RULE FROM information_schema.KEY_COLUMN_USAGE k JOIN information_schema.REFERENTIAL_CONSTRAINTS r ON r.CONSTRAINT_SCHEMA=k.CONSTRAINT_SCHEMA AND r.TABLE_NAME=k.TABLE_NAME AND r.CONSTRAINT_NAME=k.CONSTRAINT_NAME WHERE k.TABLE_SCHEMA=DATABASE() AND k.TABLE_NAME=? AND k.REFERENCED_TABLE_NAME IS NOT NULL ORDER BY k.CONSTRAINT_NAME, k.ORDINAL_POSITION")
            .bind(&name).fetch_all(&mut connection).await?;
        let mut keys = BTreeMap::<String, ForeignKey>::new();
        for row in rows {
            let key = keys
                .entry(row.try_get("CONSTRAINT_NAME")?)
                .or_insert(ForeignKey {
                    columns: vec![],
                    target: row.try_get("REFERENCED_TABLE_NAME")?,
                    target_columns: vec![],
                    on_delete: row.try_get("DELETE_RULE")?,
                    on_update: row.try_get("UPDATE_RULE")?,
                });
            key.columns.push(row.try_get("COLUMN_NAME")?);
            key.target_columns
                .push(row.try_get("REFERENCED_COLUMN_NAME")?);
        }
        tables.push(Table {
            name,
            columns,
            foreign_keys: keys.into_values().collect(),
        });
    }
    connection.close().await?;
    Ok(tables)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_schema_is_read_only_and_includes_manual_tables_and_composite_keys() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = Project::new("schema-test".into(), 3000, vec![]);
        config.stack = Database::Sqlite.stack().into();
        fs::write(
            dir.path().join("miblo.yaml"),
            serde_yaml::to_string(&config).unwrap(),
        )
        .unwrap();
        fs::write(
            dir.path().join(".env"),
            "DATABASE_URL=sqlite://miblo.sqlite?mode=rwc\n",
        )
        .unwrap();
        assert!(inspect(dir.path()).is_err());
        assert!(!dir.path().join("miblo.sqlite").exists());
        fs::create_dir(dir.path().join("migrations")).unwrap();
        fs::write(dir.path().join("migrations/000001_test.sql"), "CREATE TABLE parents (a INTEGER NOT NULL, b INTEGER NOT NULL, PRIMARY KEY(a,b)); CREATE TABLE children (id INTEGER PRIMARY KEY, a INTEGER, b INTEGER, FOREIGN KEY(a,b) REFERENCES parents(a,b) ON DELETE SET NULL); CREATE TABLE sqlitex (id INTEGER PRIMARY KEY DESC);").unwrap();
        run(dir.path(), Action::Init).unwrap();
        let schema = inspect(dir.path()).unwrap();
        assert_eq!(schema.tables.len(), 3);
        let child = &schema.tables[0];
        assert_eq!(child.name, "children");
        assert_eq!(child.columns[0].primary_key, 1);
        assert!(!child.columns[0].nullable);
        assert!(child.columns[1].nullable);
        assert_eq!(child.foreign_keys[0].columns, ["a", "b"]);
        assert_eq!(child.foreign_keys[0].target, "parents");
        assert_eq!(child.foreign_keys[0].target_columns, ["a", "b"]);
        assert_eq!(child.foreign_keys[0].on_delete, "SET NULL");
        assert_eq!(schema.tables[1].columns[1].primary_key, 2);
        assert_eq!(schema.tables[2].name, "sqlitex");
        assert!(schema.tables[2].columns[0].nullable);
    }
}
