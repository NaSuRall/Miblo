use super::*;
use crate::config::Field;
use std::process::{Command, Stdio};

/// Additive migrations never rebuild a table or disable foreign key checks.
/// Nullable columns leave existing rows unchanged, including unique columns.
pub fn alteration_sql(resource: &Resource, fields: &[Field], database: Database) -> Result<String> {
    ensure!(!fields.is_empty(), "Supply at least one new optional field");
    resource.validate_identity()?;
    Resource::new(resource.name.clone(), fields.to_vec()).validate()?;
    ensure!(
        fields.iter().all(|f| f.nullable),
        "New fields on an existing model must be optional (use type?, e.g. nickname:string?). Existing rows keep NULL"
    );
    let table = resource.table();
    let mut columns = Vec::new();
    let mut indexes = Vec::new();
    let mut foreign_keys = Vec::new();
    for field in fields {
        let name = &field.name;
        let key = &digest(format!("{table}:{name}").as_bytes())[..16];
        let mut column = format!("ADD COLUMN `{name}` {} NULL", field.kind.sql(database));
        if let Some(reference) = &field.references {
            let target = Resource::new(reference.resource.clone(), vec![]).table();
            let clause = format!(
                "REFERENCES `{target}` (`id`) ON DELETE {}",
                reference.on_delete.sql()
            );
            if database == Database::Sqlite {
                column.push_str(&format!(" {clause}"));
            } else {
                foreign_keys.push(format!(
                    "ADD CONSTRAINT `miblo_fk_{key}` FOREIGN KEY (`{name}`) {clause}"
                ));
            }
        }
        columns.push(column);
        if field.unique || field.references.is_some() {
            let unique = if field.unique { "UNIQUE " } else { "" };
            let index = format!("miblo_add_{key}");
            indexes.push(if database == Database::Sqlite {
                format!("CREATE {unique}INDEX `{index}` ON `{table}` (`{name}`);")
            } else {
                format!("ADD {unique}INDEX `{index}` (`{name}`)")
            });
        }
    }
    let mut sql =
        "-- Add optional fields; existing rows retain their values and new columns are NULL.\n"
            .to_owned();
    if database == Database::Sqlite {
        for column in columns {
            sql.push_str(&format!("ALTER TABLE `{table}` {column};\n"));
        }
        for index in indexes {
            sql.push_str(&format!("{index}\n"));
        }
    } else {
        // One ALTER statement: MySQL DDL implicitly commits; separate statements
        // could leave half an alteration applied on failure.
        columns.extend(indexes);
        columns.extend(foreign_keys);
        sql.push_str(&format!(
            "ALTER TABLE `{table}`\n    {};\n",
            columns.join(",\n    ")
        ));
    }
    Ok(sql)
}

pub fn edit(
    root: &Path,
    name: &str,
    fields: Vec<Field>,
    offline: bool,
    cache: &Path,
    dry_run: bool,
) -> Result<()> {
    let _lock = if dry_run {
        None
    } else {
        Some(OperationLock::acquire(root)?)
    };
    let mut originals = BTreeMap::new();
    for path in [CONFIG, STATE] {
        originals.insert(path.into(), fs::read(safe_path(root, path)?)?);
    }
    let (mut project, state) = inspect(root)?;
    let index = project
        .resources
        .iter()
        .position(|r| r.name == name)
        .with_context(|| format!("Unknown model: {name}"))?;
    let old = project.resources[index].clone();
    let sql = alteration_sql(&old, &fields, project.database())?;
    project.resources[index].fields.extend(fields);
    project.validate()?; // includes duplicate fields, target models and total field limit
    let updated = &project.resources[index];
    let pack = Pack::pinned(&state.template, offline, cache)?;
    ensure!(
        pack.id == project.stack,
        "Template stack does not match the project"
    );
    ensure!(
        !updated.fields.iter().any(|f| f.references.is_some())
            || pack.features.iter().any(|f| f == "relations"),
        "This template does not support relations; update it with miblo template update first"
    );
    let next = next_migration(root, state.next_migration)?;
    let mut generated = state.generated;
    let mut files = Files::new();
    for (template, file) in RESOURCE_FILES {
        let path = format!("src/resources/{}/{file}", old.module());
        let current = fs::read_to_string(safe_path(root, &path)?)?;
        let ancestor = match generated.get(&path) {
            Some(source) => source.clone(),
            // Projects created before 0.6 have no saved generation baseline.
            None => pack.render(template, &resource_data(&old, project.database()))?,
        };
        let proposed = pack.render(template, &resource_data(updated, project.database()))?;
        if ancestor != proposed {
            let merged = merge_source(&ancestor, &current, &proposed)
                .with_context(|| format!("Cannot safely update {path}; no files were changed. Keep your custom code and resolve the overlapping changes before retrying"))?;
            originals.insert(path.clone(), current.into_bytes());
            files.insert(path.clone(), merged);
        }
        generated.insert(path, proposed);
    }
    let migration = format!("migrations/{next:06}_alter_{}.sql", old.table());
    files.insert(migration.clone(), sql.clone());
    // metadata() normally records freshly rendered sources. Here the merged
    // files include user edits, so preserve the separate pristine ancestor.
    let mut meta = Files::new();
    metadata(
        &pack,
        &project,
        next.checked_add(1).context("Migration counter overflow")?,
        generated,
        &mut meta,
    )?;
    for path in [CONFIG, STATE] {
        files.insert(path.into(), meta.remove(path).context("Missing metadata")?);
    }
    // The registry does not change for an edit; preserve formatting byte-for-byte.
    let backup = format!(".miblo/backups/{next:06}");
    for (path, bytes) in &originals {
        files.insert(
            format!("{backup}/{path}"),
            String::from_utf8(bytes.clone())?,
        );
    }
    preflight(root, &files, &originals)?;
    println!("Migration: {migration}\n{sql}");
    if dry_run {
        show_plan(&files);
        return Ok(());
    }
    commit(root, &files, &originals)?;
    println!("Updated {name}. Custom code merged; replaced files saved in {backup}.");
    println!(
        "Database unchanged. Stop the API, apply with miblo db migrate (b in the interface), then rebuild/restart it. Previous rows keep NULL in the new fields."
    );
    Ok(())
}

fn merge_source(ancestor: &str, current: &str, proposed: &str) -> Result<String> {
    let ancestor = format_source(ancestor)?;
    let current = format_source(current)?;
    let proposed = format_source(proposed)?;
    let merged = diffy::merge(&ancestor, &current, &proposed)
        .map_err(|_| anyhow::anyhow!("Custom code conflicts with the generated changes"))?;
    format_source(&merged)
}

fn format_source(source: &str) -> Result<String> {
    // An isolated directory avoids loading project rustfmt config or writing project files. Files are formatted only in this temporary directory.
    let temporary = tempfile::tempdir()?;
    fs::write(temporary.path().join("rustfmt.toml"), "edition = '2024'\n")?;
    let mut input = tempfile::tempfile()?;
    input.write_all(source.as_bytes())?;
    use std::io::{Seek, SeekFrom};
    input.seek(SeekFrom::Start(0))?;
    let output = Command::new("rustfmt").args(["--emit", "stdout", "--config-path"])
        .arg(temporary.path().join("rustfmt.toml")).current_dir(temporary.path())
        .stdin(Stdio::from(input)).output()
        .context("rustfmt is needed to merge model code safely; install it with rustup component add rustfmt")?;
    ensure!(
        output.status.success(),
        "Cannot format model code: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_custom_logic_and_rejects_overlapping_changes() {
        let base = "pub struct User {pub id:i64}\nfn rule() -> i32 { 1 }\n";
        let custom =
            "// business rules\npub struct User { pub id: i64 }\nfn rule() -> i32 { 42 }\n";
        let next =
            "pub struct User {pub id:i64,pub nickname:Option<String>}\nfn rule() -> i32 { 1 }\n";
        let result = merge_source(base, custom, next).unwrap();
        assert!(
            result.contains("business rules")
                && result.contains("42")
                && result.contains("nickname")
        );
        assert!(merge_source(base, custom, &next.replace("{ 1 }", "{ 3 }")).is_err());
    }
}
