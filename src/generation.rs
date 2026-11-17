use crate::{
    config::{Database, FieldType, Project, Resource},
    templates::{PROJECT_FILES, Pack, RESOURCE_FILES, TemplateLock, digest},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

const REGISTRY: &str = "src/resources/mod.rs";
const CONFIG: &str = "miblo.yaml";
const STATE: &str = ".miblo/state.json";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub schema_version: u32,
    pub template: TemplateLock,
    pub config_sha256: String,
    pub registry_sha256: String,
    pub next_migration: u64,
}

type Files = BTreeMap<String, String>;

fn project_data(project: &Project) -> Value {
    json!({ "project_name": project.name, "crate_name": project.name.replace('-', "_"), "port": project.port,
        "resources": project.resources.iter().map(|r| json!({"module": r.module()})).collect::<Vec<_>>() })
}

fn resource_data(resource: &Resource, database: Database) -> Value {
    json!({"name": resource.name, "module": resource.module(), "table": resource.table(), "path": resource.path,
        "fields": resource.fields.iter().map(|f| json!({
            "name": f.name, "rust_type": f.kind.rust(), "sql_type": f.kind.sql(database),
            "nullable": f.nullable, "unique": f.unique, "private": f.private,
            "is_string": f.kind == FieldType::String, "is_text": f.kind == FieldType::Text,
            "string_like": matches!(f.kind, FieldType::String | FieldType::Text),
        })).collect::<Vec<_>>() })
}

fn resource_files(
    pack: &Pack,
    resource: &Resource,
    migration: u64,
    database: Database,
    files: &mut Files,
) -> Result<()> {
    let data = resource_data(resource, database);
    for (template, file) in RESOURCE_FILES {
        files.insert(
            format!("src/resources/{}/{file}", resource.module()),
            pack.render(template, &data)?,
        );
    }
    files.insert(
        format!("migrations/{migration:06}_create_{}.sql", resource.table()),
        pack.render("resource/migration.sql.hbs", &data)?,
    );
    Ok(())
}

fn metadata(pack: &Pack, project: &Project, next: u64, files: &mut Files) -> Result<()> {
    let config = serde_yaml::to_string(project)?;
    let registry = pack.render("registry.rs.hbs", &project_data(project))?;
    let state = State {
        schema_version: 1,
        template: pack.lock.clone(),
        config_sha256: digest(config.as_bytes()),
        registry_sha256: registry_digest(registry.as_bytes()),
        next_migration: next,
    };
    files.insert(CONFIG.into(), config);
    files.insert(REGISTRY.into(), registry);
    files.insert(STATE.into(), serde_json::to_string_pretty(&state)? + "\n");
    Ok(())
}

pub fn create(project: &Project, pack: &Pack, destination: &Path, dry_run: bool) -> Result<()> {
    project.validate()?;
    ensure!(
        pack.id == project.stack,
        "Template stack {} does not match {}",
        pack.id,
        project.stack
    );
    ensure!(
        !exists(destination)?,
        "Destination already exists; nothing was changed: {}",
        destination.display()
    );
    let mut files = Files::new();
    for (template, output) in PROJECT_FILES {
        if project.database() == Database::Sqlite && *output == "compose.yaml" {
            continue;
        }
        files.insert(
            (*output).into(),
            pack.render(template, &project_data(project))?,
        );
    }
    for (index, resource) in project.resources.iter().enumerate() {
        resource_files(
            pack,
            resource,
            index as u64 + 1,
            project.database(),
            &mut files,
        )?;
    }
    // Keep migration directory present even when starting without a resource.
    files.insert("migrations/.gitkeep".into(), String::new());
    if project.database() == Database::Sqlite {
        files.insert(
            ".env".into(),
            pack.render("project/env.hbs", &project_data(project))?,
        );
    }
    metadata(
        pack,
        project,
        project.resources.len() as u64 + 1,
        &mut files,
    )?;
    if dry_run {
        show_plan(&files);
        return Ok(());
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    ensure!(
        parent.is_dir(),
        "Destination parent does not exist: {}",
        parent.display()
    );
    // Claim a new directory atomically. No existing project can be overwritten.
    fs::create_dir(destination)?;
    if let Err(error) = commit(destination, &files, &BTreeMap::new()) {
        // commit only removes files it created; do not delete unrelated files.
        let _ = fs::remove_dir(destination);
        return Err(error);
    }
    println!(
        "Created {} (Rust / Axum / {}).",
        destination.display(),
        project.database().name()
    );
    if project.database() == Database::Sqlite {
        println!(
            "Next: cd {}\n      cargo run\nSQLite is configured automatically; the database and tables are created at startup.",
            destination.display()
        );
    } else {
        println!(
            "Next: cd {}\n      cp .env.example .env\n      docker compose up -d db\n      cargo run",
            destination.display()
        );
    }
    Ok(())
}

pub fn inspect(root: &Path) -> Result<(Project, State)> {
    safe_path(root, CONFIG)?;
    safe_path(root, STATE)?;
    safe_path(root, REGISTRY)?;
    let project = Project::load(&root.join(CONFIG))?;
    let state: State = serde_json::from_slice(&fs::read(root.join(STATE)).context(
        "Missing Miblo state; only projects created by Miblo 0.2 or later can be extended",
    )?)?;
    ensure!(state.schema_version == 1, "Unsupported Miblo state version");
    ensure!(state.next_migration > 0, "Invalid migration counter");
    ensure!(
        digest(serde_yaml::to_string(&project)?.as_bytes()) == state.config_sha256,
        "miblo.yaml was modified outside Miblo. Restore its previous resource definitions before adding a resource; existing schema edits are not yet supported"
    );
    ensure!(
        registry_digest(&fs::read(root.join(REGISTRY))?) == state.registry_sha256,
        "src/resources/mod.rs was customized. Move custom route registration to src/lib.rs and restore the generated registry before adding a resource; nothing was changed"
    );
    Ok((project, state))
}

pub fn add(
    root: &Path,
    resource: Resource,
    offline: bool,
    cache: &Path,
    dry_run: bool,
) -> Result<()> {
    resource.validate()?;
    let _lock = if dry_run {
        None
    } else {
        Some(OperationLock::acquire(root)?)
    };
    let (mut project, state) = inspect(root)?;
    ensure!(
        !project
            .resources
            .iter()
            .any(|r| r.module() == resource.module()),
        "Resource {} already exists; nothing was changed",
        resource.name
    );
    project.resources.push(resource.clone());
    project.validate()?;
    let pack = Pack::pinned(&state.template, offline, cache)?;
    ensure!(
        pack.id == project.stack,
        "Template stack does not match the project"
    );
    let mut files = Files::new();
    // Manual migrations are allowed; the next generated migration follows their numeric version.
    let next = next_migration(root, state.next_migration)?;
    resource_files(&pack, &resource, next, project.database(), &mut files)?;
    metadata(
        &pack,
        &project,
        next.checked_add(1).context("Migration counter overflow")?,
        &mut files,
    )?;
    let mut originals = BTreeMap::new();
    for path in [CONFIG, STATE, REGISTRY] {
        originals.insert(path.into(), fs::read(root.join(path))?);
    }
    preflight(root, &files, &originals)?;
    if dry_run {
        show_plan(&files);
        return Ok(());
    }
    commit(root, &files, &originals)?;
    println!(
        "Added {} at {}. Existing resource files were preserved.",
        resource.name, resource.path
    );
    println!("Restart the API to apply the new migration.");
    Ok(())
}

fn next_migration(root: &Path, minimum: u64) -> Result<u64> {
    safe_path(root, "migrations")?;
    let mut next = minimum;
    for entry in fs::read_dir(root.join("migrations"))? {
        let name = entry?.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".sql") {
            let version = name
                .split('_')
                .next()
                .context("Invalid migration name")?
                .parse::<u64>()
                .context("Migration names must start with a numeric version followed by '_'")?;
            ensure!(
                version < i64::MAX as u64,
                "Migration version exceeds SQLx range"
            );
            next = next.max(version + 1);
        }
    }
    ensure!(
        next < i64::MAX as u64,
        "Migration version exceeds SQLx range"
    );
    Ok(next)
}

fn exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn safe_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        ensure!(
            matches!(component, Component::Normal(_)),
            "Invalid output path: {relative}"
        );
        path.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            ensure!(
                !metadata.file_type().is_symlink(),
                "Refusing symbolic link: {}",
                path.display()
            );
        }
    }
    Ok(path)
}

fn preflight(root: &Path, files: &Files, originals: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    for name in files.keys() {
        let path = safe_path(root, name)?;
        match originals.get(name) {
            Some(expected) => ensure!(
                fs::read(&path)? == *expected,
                "File changed during generation: {name}"
            ),
            None => ensure!(
                !exists(&path)?,
                "File already exists; nothing was changed: {name}"
            ),
        }
    }
    Ok(())
}

fn commit(root: &Path, files: &Files, originals: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    preflight(root, files, originals)?;
    let mut prepared = Vec::new();
    let mut dirs = Vec::new();
    let mut written: Vec<String> = Vec::new();
    let result: Result<()> = (|| {
        for (name, content) in files {
            let path = safe_path(root, name)?;
            let parent = path.parent().context("Missing parent directory")?;
            make_dirs(root, parent, &mut dirs)?;
            let mut staged = tempfile::NamedTempFile::new_in(parent)?;
            staged.write_all(content.as_bytes())?;
            staged.as_file().sync_all()?;
            prepared.push((name.clone(), path, staged));
        }
        preflight(root, files, originals)?;
        // Write metadata last, so interrupted operations are detected by inspect().
        prepared.sort_by_key(|(name, _, _)| [CONFIG, REGISTRY, STATE].contains(&name.as_str()));
        for (name, path, staged) in prepared.drain(..) {
            if let Some(expected) = originals.get(&name) {
                ensure!(
                    fs::read(&path)? == *expected,
                    "File changed during generation: {name}"
                );
                staged.persist(&path)?;
            } else {
                staged.persist_noclobber(&path)?;
            }
            written.push(name);
        }
        Ok(())
    })();
    if let Err(error) = result {
        drop(prepared);
        let mut rollback_errors = Vec::new();
        for name in written.iter().rev() {
            let path = root.join(name);
            let rollback = if let Some(bytes) = originals.get(name) {
                fs::write(&path, bytes)
            } else {
                fs::remove_file(&path)
            };
            if let Err(e) = rollback {
                rollback_errors.push(format!("{name}: {e}"));
            }
        }
        for dir in dirs.iter().rev() {
            let _ = fs::remove_dir(dir);
        }
        if !rollback_errors.is_empty() {
            bail!(
                "{error:#}; rollback needs attention: {}",
                rollback_errors.join(", ")
            );
        }
        return Err(error);
    }
    Ok(())
}

fn make_dirs(root: &Path, parent: &Path, created: &mut Vec<PathBuf>) -> Result<()> {
    let mut path = root.to_path_buf();
    for component in parent.strip_prefix(root)?.components() {
        path.push(component);
        if !exists(&path)? {
            fs::create_dir(&path)?;
            created.push(path.clone());
        }
        ensure!(path.is_dir(), "Not a directory: {}", path.display());
    }
    Ok(())
}

fn show_plan(files: &Files) {
    println!("Dry run — planned files:");
    for name in files.keys() {
        println!("  {name}");
    }
}

struct OperationLock {
    path: PathBuf,
}
impl OperationLock {
    fn acquire(root: &Path) -> Result<Self> {
        let path = safe_path(root, ".miblo/operation.lock")?;
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path)
            .context("Cannot lock project. If a previous command was interrupted, inspect its files before removing .miblo/operation.lock")?;
        writeln!(file, "{}", std::process::id())?;
        Ok(Self { path })
    }
}
impl Drop for OperationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

// The generated registry contains no literals. Ignore whitespace-only changes
// from rustfmt, but stop on any changed token or added comment.
fn registry_digest(bytes: &[u8]) -> String {
    digest(
        &bytes
            .iter()
            .copied()
            .filter(|b| !b.is_ascii_whitespace())
            .collect::<Vec<_>>(),
    )
}
