use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

fn cli(args: &[&str], dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_miblo"))
        .args(args)
        .current_dir(dir)
        .env_remove("MIBLO_TEMPLATE_DIR")
        .env_remove("MIBLO_TEMPLATE_BASE_URL")
        .env_remove("DATABASE_URL")
        .output()
        .unwrap()
}
fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn failure(output: Output, message: &str) {
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(message),
        "expected {message:?}; got {stderr}"
    );
    assert!(!stderr.contains("panicked at"));
}

// Tiny runtime fixtures test the CLI contract without bundling production templates.
fn pack(dir: &Path) -> PathBuf {
    let root = dir.join("pack");
    fs::create_dir_all(root.join("project")).unwrap();
    fs::create_dir_all(root.join("resource")).unwrap();
    fs::write(
        root.join("manifest.json"),
        r#"{"schema_version":1,"id":"rust-axum-mysql","version":"0.1.0"}"#,
    )
    .unwrap();
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "main.rs",
        "lib.rs",
        "error.rs",
        "patch.rs",
        "build.rs",
        "env",
        "gitignore",
        "README.md",
        "compose.yaml",
        "health.rs",
    ] {
        fs::write(
            root.join(format!("project/{name}.hbs")),
            "project={{project_name}}\n",
        )
        .unwrap();
    }
    for name in ["mod.rs", "model.rs", "handlers.rs", "migration.sql"] {
        fs::write(
            root.join(format!("resource/{name}.hbs")),
            "resource={{name}}\n",
        )
        .unwrap();
    }
    fs::write(
        root.join("registry.rs.hbs"),
        "{{#each resources}}pub mod {{module}};\n{{/each}}",
    )
    .unwrap();
    root
}
fn create(dir: &Path) -> PathBuf {
    let template = pack(dir);
    success(cli(
        &[
            "new",
            "sample",
            "--template-dir",
            template.to_str().unwrap(),
            "--resource",
            "Product",
            "--field",
            "title:string:unique",
            "--yes",
        ],
        dir,
    ));
    dir.join("sample")
}
fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(tree(&path));
        } else {
            files.push((path.clone(), fs::read(path).unwrap()));
        }
    }
    files.sort();
    files
}

fn editable_project(dir: &Path) -> PathBuf {
    let template = pack(dir);
    fs::write(template.join("manifest.json"), r#"{"schema_version":1,"id":"rust-axum-sqlite","version":"0.3.0","features":["relations"]}"#).unwrap();
    fs::write(
        template.join("resource/mod.rs.hbs"),
        "pub mod model;\npub mod handlers;\n",
    )
    .unwrap();
    fs::write(template.join("resource/model.rs.hbs"), "pub struct {{name}} {pub id:i64,{{#each fields}}pub {{name}}:{{#if nullable}}Option<{{rust_type}}>{{else}}{{rust_type}}{{/if}},{{/each}}\n}\n").unwrap();
    fs::write(
        template.join("resource/handlers.rs.hbs"),
        "pub fn columns() -> Vec<&'static str> { vec![{{#each fields}}\"{{name}}\",{{/each}}] }\n",
    )
    .unwrap();
    fs::write(template.join("resource/migration.sql.hbs"), "CREATE TABLE `{{table}}` (`id` INTEGER PRIMARY KEY,{{#each fields}}`{{name}}` {{sql_type}}{{#unless nullable}} NOT NULL{{/unless}}{{#unless @last}},{{/unless}}{{/each}});\n").unwrap();
    success(cli(
        &[
            "new",
            "sample",
            "--database",
            "sqlite",
            "--template-dir",
            template.to_str().unwrap(),
            "--resource",
            "User",
            "--field",
            "name:string",
            "--yes",
        ],
        dir,
    ));
    let root = dir.join("sample");
    fs::write(root.join(".env"), "DATABASE_URL=sqlite://data.sqlite\n").unwrap();
    root
}

#[test]
fn edit_preserves_history_custom_code_and_legacy_projects() {
    let dir = TempDir::new().unwrap();
    let root = editable_project(dir.path());
    // Exercise the upgrade path for projects created before generation baselines.
    let path = root.join(".miblo/state.json");
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state.as_object_mut().unwrap().remove("generated");
    state.as_object_mut().unwrap().remove("resource_templates");
    fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
    let model = root.join("src/resources/user/model.rs");
    let mut source = fs::read_to_string(&model).unwrap();
    source.push_str("\n// business logic\nimpl User {pub fn label(&self)->&str { &self.name }}\n");
    fs::write(&model, &source).unwrap();
    let before = tree(&root);
    success(cli(
        &["edit", "User", "nickname:string?:unique", "--dry-run"],
        &root,
    ));
    assert_eq!(tree(&root), before, "dry run must not write any files");
    success(cli(
        &[
            "edit",
            "User",
            "nickname:string?:unique",
            "--yes",
            "--offline",
        ],
        &root,
    ));
    assert!(
        fs::read_to_string(&model)
            .unwrap()
            .contains("business logic")
    );
    assert!(
        fs::read_to_string(&model)
            .unwrap()
            .contains("nickname: Option<String>")
    );
    assert_eq!(
        fs::read_to_string(root.join(".miblo/backups/000002/src/resources/user/model.rs")).unwrap(),
        source
    );
    for (path, bytes) in before
        .iter()
        .filter(|(path, _)| path.parent() == Some(root.join("migrations").as_path()))
    {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
    success(cli(&["add", "Group", "name:string", "--yes"], &root));
    success(cli(
        &[
            "edit",
            "User",
            "group_id:int?:ref=Group:on_delete=set_null",
            "--yes",
        ],
        &root,
    ));
    let sql = fs::read_to_string(root.join("migrations/000004_alter_users.sql")).unwrap();
    assert!(sql.contains("REFERENCES `groups` (`id`) ON DELETE SET NULL"));
    assert!(!sql.contains("DROP") && !sql.contains("CREATE TABLE"));
    success(cli(&["db", "init"], &root));
    success(cli(&["db", "migrate"], &root));
    success(cli(&["doctor"], &root));
    assert!(
        fs::read_to_string(&model)
            .unwrap()
            .contains("business logic")
    );
}

#[test]
fn invalid_edits_and_merge_conflicts_write_nothing() {
    let dir = TempDir::new().unwrap();
    let root = editable_project(dir.path());
    for (arguments, error) in [
        (
            vec!["edit", "User", "nickname:string", "--yes"],
            "must be optional",
        ),
        (
            vec!["edit", "User", "name:string?", "--yes"],
            "Duplicate field",
        ),
        (
            vec!["edit", "User", "group_id:int?:ref=Missing", "--yes"],
            "create that model first",
        ),
        (vec!["edit", "User", "--yes"], "at least one"),
        (
            vec!["edit", "Missing", "note:string?", "--yes"],
            "Unknown model",
        ),
    ] {
        let before = tree(&root);
        failure(cli(&arguments, &root), error);
        assert_eq!(tree(&root), before);
    }
    fs::write(
        root.join("src/resources/user/model.rs"),
        "// Handwritten replacement\npub fn custom() {}\n",
    )
    .unwrap();
    let before = tree(&root);
    failure(
        cli(&["edit", "User", "nickname:string?", "--yes"], &root),
        "Cannot safely update",
    );
    assert_eq!(tree(&root), before, "conflict must leave all files intact");
}

#[test]
fn failed_sqlite_alteration_rolls_back_columns_and_keeps_rows() {
    use sqlx::Connection;
    let dir = TempDir::new().unwrap();
    let root = editable_project(dir.path());
    success(cli(&["db", "init"], &root));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let options = sqlx::sqlite::SqliteConnectOptions::new().filename(root.join("data.sqlite"));
    runtime.block_on(async {
        let mut connection = sqlx::SqliteConnection::connect_with(&options).await.unwrap();
        sqlx::raw_sql("INSERT INTO users(id,name) VALUES (42,'kept'); ALTER TABLE users ADD COLUMN manual TEXT; UPDATE users SET manual='custom';")
            .execute(&mut connection).await.unwrap();
        connection.close().await.unwrap();
    });
    success(cli(
        &[
            "edit",
            "User",
            "nickname:string?",
            "manual:string?",
            "--yes",
        ],
        &root,
    ));
    failure(cli(&["db", "migrate"], &root), "duplicate column name");
    runtime.block_on(async {
        let mut connection = sqlx::SqliteConnection::connect_with(&options)
            .await
            .unwrap();
        let row: (i64, String, String) = sqlx::query_as("SELECT id,name,manual FROM users")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(row, (42, "kept".into(), "custom".into()));
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pragma_table_info('users') WHERE name='nickname'",
        )
        .fetch_one(&mut connection)
        .await
        .unwrap();
        assert_eq!(
            count, 0,
            "The first ADD COLUMN must roll back when the second fails"
        );
        let versions: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(versions, 1);
        connection.close().await.unwrap();
    });
}

fn layered_pack(dir: &Path) -> PathBuf {
    let root = pack(dir);
    fs::write(root.join("manifest.json"), r#"{"schema_version":2,"id":"rust-axum-sqlite","version":"0.4.0","features":["relations","layered-resources"]}"#).unwrap();
    for file in [
        "main", "lib", "error", "patch", "build", "health", "config", "database",
    ] {
        fs::write(
            root.join(format!("project/{file}.rs.hbs")),
            "// Project fixture\n",
        )
        .unwrap();
    }
    fs::write(
        root.join("project/env.hbs"),
        "DATABASE_URL=sqlite://data.sqlite\n",
    )
    .unwrap();
    fs::write(
        root.join("resource/mod.rs.hbs"),
        "pub mod model; pub mod handlers; pub mod requests; pub mod service; mod repository;\n",
    )
    .unwrap();
    for file in ["model", "requests"] {
        fs::write(root.join(format!("resource/{file}.rs.hbs")), "pub struct {{name}} {\n{{#each fields}}pub {{name}}:{{#if nullable}}Option<{{rust_type}}>{{else}}{{rust_type}}{{/if}},\n{{/each}}}\n").unwrap();
    }
    fs::write(
        root.join("resource/handlers.rs.hbs"),
        "pub fn handler() {}\n",
    )
    .unwrap();
    fs::write(
        root.join("resource/service.rs.hbs"),
        "pub fn business_rule() -> i32 { 1 }\n",
    )
    .unwrap();
    fs::write(
        root.join("resource/repository.rs.hbs"),
        "pub fn columns() -> Vec<&'static str> { vec![{{#each fields}}\"{{name}}\",{{/each}}] }\n",
    )
    .unwrap();
    fs::write(root.join("resource/migration.sql.hbs"), "CREATE TABLE `{{table}}` (`id` INTEGER PRIMARY KEY,{{#each fields}}`{{name}}` {{sql_type}}{{#unless nullable}} NOT NULL{{/unless}}{{#unless @last}},{{/unless}}{{/each}});\n").unwrap();
    root
}

#[test]
fn template_updates_keep_each_models_architecture_and_custom_service() {
    let dir = TempDir::new().unwrap();
    let root = editable_project(dir.path());
    let template = layered_pack(&dir.path().join("next"));
    let before = tree(&root);
    success(cli(
        &[
            "template",
            "update",
            "--template-dir",
            template.to_str().unwrap(),
        ],
        &root,
    ));
    for (path, bytes) in before
        .iter()
        .filter(|(path, _)| *path != root.join(".miblo/state.json"))
    {
        assert_eq!(fs::read(path).unwrap(), *bytes);
    }
    success(cli(&["add", "Group", "label:string", "--yes"], &root));
    assert!(!root.join("src/resources/user/service.rs").exists());
    let service = root.join("src/resources/group/service.rs");
    let custom = "// my business rules\npub fn business_rule() -> i32 { 42 }\n";
    fs::write(&service, custom).unwrap();
    // The old model is edited with its old pack, and can reference a newer model.
    success(cli(
        &["edit", "User", "group_id:int?:ref=Group", "--yes"],
        &root,
    ));
    assert!(!root.join("src/resources/user/requests.rs").exists());
    success(cli(
        &["edit", "Group", "owner_id:int?:ref=User", "--yes"],
        &root,
    ));
    assert_eq!(fs::read_to_string(service).unwrap(), custom);
    assert!(
        fs::read_to_string(root.join("src/resources/group/requests.rs"))
            .unwrap()
            .contains("owner_id")
    );
    assert!(
        fs::read_to_string(root.join("src/resources/group/repository.rs"))
            .unwrap()
            .contains("owner_id")
    );
    success(cli(&["db", "init"], &root));
    success(cli(&["doctor"], &root));
    // Re-selecting the pack is compatible even after module sorting by rustfmt.
    success(cli(
        &[
            "template",
            "update",
            "--template-dir",
            template.to_str().unwrap(),
        ],
        &root,
    ));
}

#[test]
fn layered_archive_generates_formatted_sources_and_works_offline() {
    let dir = TempDir::new().unwrap();
    let template = layered_pack(dir.path());
    let server = Server::new(archive(&template, false), None);
    let cache = dir.path().join("cache");
    success(cli(
        &[
            "new",
            "sample",
            "--database",
            "sqlite",
            "--resource",
            "User",
            "--field",
            "name:string",
            "--template-url",
            &server.url,
            "--template-version",
            "0.4.0",
            "--cache-dir",
            cache.to_str().unwrap(),
            "--yes",
        ],
        dir.path(),
    ));
    drop(server);
    let root = dir.path().join("sample");
    assert!(root.join("src/config.rs").exists() && root.join("src/database.rs").exists());
    assert!(
        fs::read_to_string(root.join("src/resources/user/model.rs"))
            .unwrap()
            .contains("    pub name: String,")
    );
    success(cli(
        &[
            "edit",
            "User",
            "nickname:string?",
            "--offline",
            "--cache-dir",
            cache.to_str().unwrap(),
            "--yes",
        ],
        &root,
    ));
    let broken = layered_pack(&dir.path().join("broken"));
    fs::remove_file(broken.join("resource/service.rs.hbs")).unwrap();
    let before = tree(&root);
    failure(
        cli(
            &[
                "template",
                "update",
                "--template-dir",
                broken.to_str().unwrap(),
            ],
            &root,
        ),
        "No such file",
    );
    assert_eq!(tree(&root), before);
}

#[test]
fn help_and_errors_use_the_miblo_binary() {
    let dir = TempDir::new().unwrap();
    let help = success(cli(&["--help"], dir.path()));
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage: miblo"));
    failure(
        cli(&["run", "--project", "missing"], dir.path()),
        "No Cargo.toml",
    );
    failure(
        cli(&["new", "../escape", "--yes"], dir.path()),
        "Project name",
    );
    failure(
        cli(
            &["new", "sample", "--template-dir", "missing", "--yes"],
            dir.path(),
        ),
        "Template directory not found",
    );
    assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[test]
fn resource_add_preserves_customizations_and_uses_new_migrations() {
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    let custom = root.join("src/resources/product/handlers.rs");
    fs::write(&custom, "// handwritten business logic\n").unwrap();
    fs::write(root.join("migrations/000050_manual.sql"), "-- custom\n").unwrap();
    success(cli(
        &[
            "add",
            "resource",
            "Category",
            "--field",
            "label:string",
            "--yes",
        ],
        &root,
    ));
    assert_eq!(
        fs::read_to_string(custom).unwrap(),
        "// handwritten business logic\n"
    );
    assert!(
        root.join("migrations/000051_create_categories.sql")
            .is_file()
    );
    let routes = fs::read_to_string(root.join("src/resources/mod.rs")).unwrap();
    assert_eq!(routes.matches("pub mod product;").count(), 1);
    assert_eq!(routes.matches("pub mod category;").count(), 1);
    let before = tree(&root);
    failure(
        cli(
            &[
                "add",
                "resource",
                "Category",
                "--field",
                "label:string",
                "--yes",
            ],
            &root,
        ),
        "already exists",
    );
    assert_eq!(tree(&root), before);
}

#[test]
fn customized_registry_blocks_add_without_changing_files() {
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    fs::write(root.join("src/resources/mod.rs"), "// custom registry\n").unwrap();
    let before = tree(&root);
    failure(
        cli(
            &["add", "resource", "Order", "--field", "total:int", "--yes"],
            &root,
        ),
        "was customized",
    );
    assert_eq!(tree(&root), before);
}

#[test]
fn dry_runs_and_output_conflicts_leave_project_intact() {
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    let before = tree(&root);
    success(cli(
        &[
            "add",
            "resource",
            "Order",
            "--field",
            "total:int",
            "--yes",
            "--dry-run",
        ],
        &root,
    ));
    assert_eq!(tree(&root), before);
    fs::create_dir_all(root.join("src/resources/order")).unwrap();
    fs::write(root.join("src/resources/order/model.rs"), "// keep\n").unwrap();
    let before = tree(&root);
    failure(
        cli(
            &["add", "resource", "Order", "--field", "total:int", "--yes"],
            &root,
        ),
        "File already exists",
    );
    assert_eq!(tree(&root), before);
    failure(
        cli(&["new", "sample", "--yes"], dir.path()),
        "Destination exists",
    );
    assert_eq!(tree(&root), before);
}

#[test]
fn changed_template_and_invalid_fields_are_rejected() {
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    let before = tree(&root);
    failure(
        cli(
            &["add", "resource", "Order", "--field", "id:int", "--yes"],
            &root,
        ),
        "id is generated",
    );
    failure(
        cli(
            &[
                "add",
                "resource",
                "Order",
                "--field",
                "type:string",
                "--yes",
            ],
            &root,
        ),
        "reserved identifier",
    );
    failure(
        cli(
            &[
                "add",
                "resource",
                "Order",
                "--field",
                "note:text:unique",
                "--yes",
            ],
            &root,
        ),
        "Use string",
    );
    fs::write(dir.path().join("pack/resource/model.rs.hbs"), "changed").unwrap();
    failure(
        cli(
            &["add", "resource", "Order", "--field", "total:int", "--yes"],
            &root,
        ),
        "miblo template update",
    );
    assert_eq!(tree(&root), before);
}

#[cfg(unix)]
#[test]
fn symlink_output_is_rejected() {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    let outside = dir.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, root.join("src/resources/order")).unwrap();
    failure(
        cli(
            &["add", "resource", "Order", "--field", "total:int", "--yes"],
            &root,
        ),
        "symbolic link",
    );
    assert!(fs::read_dir(outside).unwrap().next().is_none());
}

fn archive(root: &Path, link: bool) -> Vec<u8> {
    use flate2::{Compression, write::GzEncoder};
    let gz = GzEncoder::new(Vec::new(), Compression::default());
    let mut tar = tar::Builder::new(gz);
    for (path, bytes) in tree(root) {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(
            &mut header,
            path.strip_prefix(root).unwrap(),
            bytes.as_slice(),
        )
        .unwrap();
    }
    if link {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_link_name("/tmp/outside").unwrap();
        header.set_cksum();
        tar.append_data(&mut header, "link", &[][..]).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}

struct Server {
    url: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Server {
    fn new(bytes: Vec<u8>, checksum: Option<String>) -> Self {
        use sha2::{Digest, Sha256};
        use std::{
            io::{Read, Write},
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            },
            time::Duration,
        };
        let checksum = checksum.unwrap_or_else(|| format!("{:x}", Sha256::digest(&bytes)));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/pack.tar.gz", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Accepted sockets inherit O_NONBLOCK on macOS. A partial
                        // request must not be mistaken for an archive download.
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut byte = [0; 1];
                        while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                            match stream.read(&mut byte) {
                                Ok(1) => request.push(byte[0]),
                                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                                    continue;
                                }
                                _ => break,
                            }
                        }
                        if !request.ends_with(b"\r\n\r\n") {
                            continue;
                        }
                        let is_checksum = String::from_utf8_lossy(&request)
                            .starts_with("GET /pack.tar.gz.sha256 ");
                        let content = if is_checksum {
                            checksum.as_bytes()
                        } else {
                            &bytes
                        };
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            content.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(content);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

#[test]
fn remote_template_is_pinned_cached_and_reusable_offline() {
    let dir = TempDir::new().unwrap();
    let template = pack(dir.path());
    let server = Server::new(archive(&template, false), None);
    let cache = dir.path().join("cache");
    let url = server.url.clone();
    success(cli(
        &[
            "new",
            "first",
            "--template-version",
            "0.1.0",
            "--template-url",
            &url,
            "--cache-dir",
            cache.to_str().unwrap(),
            "--yes",
        ],
        dir.path(),
    ));
    drop(server);
    success(cli(
        &[
            "new",
            "second",
            "--template-version",
            "0.1.0",
            "--template-url",
            &url,
            "--cache-dir",
            cache.to_str().unwrap(),
            "--offline",
            "--yes",
        ],
        dir.path(),
    ));
    success(cli(
        &[
            "add",
            "resource",
            "Product",
            "--field",
            "title:string",
            "--cache-dir",
            cache.to_str().unwrap(),
            "--offline",
            "--yes",
        ],
        &dir.path().join("second"),
    ));
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.path().join("second/.miblo/state.json")).unwrap())
            .unwrap();
    assert_eq!(state["template"]["sha256"].as_str().unwrap().len(), 64);
    let cached_archive = fs::read_dir(&cache)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "gz"))
        .unwrap();
    fs::write(cached_archive, "corrupt cache").unwrap();
    failure(
        cli(
            &[
                "new",
                "third",
                "--template-version",
                "0.1.0",
                "--template-url",
                &url,
                "--cache-dir",
                cache.to_str().unwrap(),
                "--offline",
                "--yes",
            ],
            dir.path(),
        ),
        "SHA-256 mismatch",
    );
    assert!(!dir.path().join("third").exists());
}

#[test]
fn download_rejects_wrong_checksum_and_archive_links() {
    let dir = TempDir::new().unwrap();
    let template = pack(dir.path());
    let cache = dir.path().join("cache");
    let server = Server::new(archive(&template, false), Some("0".repeat(64)));
    failure(
        cli(
            &[
                "new",
                "wrong",
                "--template-version",
                "0.1.0",
                "--template-url",
                &server.url,
                "--cache-dir",
                cache.to_str().unwrap(),
                "--yes",
            ],
            dir.path(),
        ),
        "SHA-256 mismatch",
    );
    drop(server);
    let server = Server::new(archive(&template, true), None);
    failure(
        cli(
            &[
                "new",
                "linked",
                "--template-version",
                "0.1.0",
                "--template-url",
                &server.url,
                "--cache-dir",
                cache.to_str().unwrap(),
                "--yes",
            ],
            dir.path(),
        ),
        "links or special files",
    );
    assert!(!dir.path().join("wrong").exists());
    assert!(!dir.path().join("linked").exists());
}

#[test]
fn changed_config_is_detected_before_adding_files() {
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    let config = root.join("miblo.yaml");
    fs::write(
        &config,
        fs::read_to_string(&config)
            .unwrap()
            .replace("title", "renamed"),
    )
    .unwrap();
    let before = tree(&root);
    failure(
        cli(
            &["add", "resource", "Order", "--field", "total:int", "--yes"],
            &root,
        ),
        "modified outside Miblo",
    );
    assert_eq!(tree(&root), before);
}

#[cfg(unix)]
#[test]
fn run_preserves_child_errors_and_returns_failure() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "").unwrap();
    let cargo = dir.path().join("cargo");
    fs::write(
        &cargo,
        "#!/bin/sh\necho 'visible compilation error' >&2\nexit 7\n",
    )
    .unwrap();
    fs::set_permissions(cargo, fs::Permissions::from_mode(0o755)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_miblo"))
        .arg("run")
        .current_dir(dir.path())
        .env("PATH", dir.path())
        .output()
        .unwrap();
    failure(output, "visible compilation error");
}

#[test]
fn short_resource_commands_and_noninteractive_help_work() {
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    success(cli(&["add", "Category", "label:string", "--yes"], &root));
    success(cli(
        &["add", "Order", "--field", "total:int", "--yes"],
        &root,
    ));
    assert!(root.join("src/resources/category/model.rs").is_file());
    assert!(root.join("src/resources/order/model.rs").is_file());
    let help = success(cli(&[], dir.path()));
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage: miblo"));
    failure(cli(&["ui"], dir.path()), "requires a terminal");
}

#[test]
fn sqlite_generates_and_migrates_tables_without_manual_configuration() {
    let dir = TempDir::new().unwrap();
    let template = pack(dir.path());
    failure(
        cli(
            &[
                "new",
                "wrong",
                "--database",
                "sqlite",
                "--template-dir",
                template.to_str().unwrap(),
                "--yes",
            ],
            dir.path(),
        ),
        "does not match",
    );
    assert!(!dir.path().join("wrong").exists());
    fs::write(
        template.join("manifest.json"),
        r#"{"schema_version":1,"id":"rust-axum-sqlite","version":"0.2.0"}"#,
    )
    .unwrap();
    fs::write(
        template.join("project/env.hbs"),
        "DATABASE_URL=sqlite://miblo.sqlite?mode=rwc\n",
    )
    .unwrap();
    fs::write(template.join("resource/migration.sql.hbs"), "CREATE TABLE `{{table}}` (id INTEGER PRIMARY KEY AUTOINCREMENT{{#each fields}}, `{{name}}` {{sql_type}}{{/each}});\n").unwrap();
    success(cli(
        &[
            "new",
            "sample",
            "--database",
            "sqlite",
            "--template-dir",
            template.to_str().unwrap(),
            "--resource",
            "User",
            "--field",
            "email:string",
            "--yes",
        ],
        dir.path(),
    ));
    let root = dir.path().join("sample");
    assert!(root.join(".env").is_file());
    assert!(!root.join("compose.yaml").exists());
    failure(cli(&["db", "check"], &root), "Connexion SQLite impossible");
    assert!(
        !root.join("miblo.sqlite").exists(),
        "check must not create the database"
    );
    success(cli(&["db", "init"], &root));
    success(cli(&["db", "migrate"], &root));
    success(cli(&["db", "check"], &root));
    success(cli(&["add", "Category", "label:string", "--yes"], &root));
    success(cli(&["db", "migrate"], &root));
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(root.join("miblo.sqlite")).read_only(true)).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('users', 'categories')").fetch_one(&pool).await.unwrap();
        assert_eq!(count, 2);
        pool.close().await;
    });
}

#[test]
fn old_projects_opt_into_relation_templates_without_rewriting_code() {
    let dir = TempDir::new().unwrap();
    let root = create(dir.path());
    let fields = ["add", "Member", "product_id:int:ref=Product", "--yes"];
    let before = tree(&root);
    failure(cli(&fields, &root), "template does not support relations");
    assert_eq!(tree(&root), before);
    let next = pack(&dir.path().join("next"));
    fs::write(
        next.join("manifest.json"),
        r#"{"schema_version":1,"id":"rust-axum-mysql","version":"0.3.0","features":["relations"]}"#,
    )
    .unwrap();
    fs::write(next.join("resource/migration.sql.hbs"), "CREATE TABLE `{{table}}` (id BIGINT PRIMARY KEY{{#each fields}}, `{{name}}` {{sql_type}}{{/each}}{{#each relations}}, FOREIGN KEY (`{{field}}`) REFERENCES `{{table}}` (id) ON DELETE {{on_delete}}{{/each}});\n").unwrap();
    fs::write(root.join("src/error.rs"), "// custom error handling\n").unwrap();
    let before = tree(&root);
    success(cli(
        &[
            "template",
            "update",
            "--template-dir",
            next.to_str().unwrap(),
        ],
        &root,
    ));
    for (path, bytes) in before {
        if path != root.join(".miblo/state.json") {
            assert_eq!(fs::read(path).unwrap(), bytes);
        }
    }
    let before = tree(&root);
    failure(
        cli(
            &["add", "Invalid", "missing_id:int:ref=Missing", "--yes"],
            &root,
        ),
        "create that model first",
    );
    assert_eq!(tree(&root), before);
    success(cli(&fields, &root));
    let migration = fs::read_to_string(root.join("migrations/000002_create_members.sql")).unwrap();
    assert!(
        migration
            .contains("FOREIGN KEY (`product_id`) REFERENCES `products` (id) ON DELETE RESTRICT")
    );
    assert_eq!(
        fs::read_to_string(root.join("src/error.rs")).unwrap(),
        "// custom error handling\n"
    );
    success(cli(&["doctor"], &root));
}

fn route_pack(dir: &Path) -> PathBuf {
    let root = layered_pack(dir);
    fs::write(root.join("manifest.json"), r#"{"schema_version":3,"id":"rust-axum-sqlite","version":"0.5.0","features":["relations","layered-resources","custom-routes"]}"#).unwrap();
    fs::write(root.join("registry.rs.hbs"), "{{#each resources}}pub mod {{module}};\n{{/each}}{{#if routes}}pub mod custom_routes;\n{{/if}}").unwrap();
    fs::create_dir(root.join("route")).unwrap();
    fs::write(
        root.join("route/registry.rs.hbs"),
        "pub mod error;\n{{#each routes}}pub mod {{name}};\n{{/each}}",
    )
    .unwrap();
    fs::write(root.join("route/error.rs.hbs"), "pub struct RouteError;\n").unwrap();
    fs::write(
        root.join("route/mod.rs.hbs"),
        "pub mod handlers; pub mod requests; pub mod service;\n",
    )
    .unwrap();
    for file in ["handlers", "requests", "service"] {
        fs::write(
            root.join(format!("route/{file}.rs.hbs")),
            "pub fn {{name}}() {}\n",
        )
        .unwrap();
    }
    root
}

#[test]
fn custom_routes_preserve_code_migrations_and_model_evolution() {
    let dir = TempDir::new().unwrap();
    let root = editable_project(dir.path());
    let before = tree(&root);
    failure(
        cli(
            &[
                "route",
                "add",
                "active_users",
                "--path",
                "/users/active",
                "--yes",
            ],
            &root,
        ),
        "miblo template update",
    );
    assert_eq!(tree(&root), before);
    let template = route_pack(&dir.path().join("routes"));
    success(cli(
        &[
            "template",
            "update",
            "--template-dir",
            template.to_str().unwrap(),
        ],
        &root,
    ));
    let before = tree(&root);
    let args = [
        "route",
        "add",
        "active_users",
        "--path",
        "/users/active",
        "--resource",
        "User",
        "--offline",
    ];
    success(cli(&[&args[..], &["--dry-run"]].concat(), &root));
    assert_eq!(tree(&root), before);
    success(cli(&[&args[..], &["--yes"]].concat(), &root));
    assert_eq!(
        tree(&root.join("migrations")),
        before
            .iter()
            .filter(|(p, _)| p.starts_with(root.join("migrations")))
            .cloned()
            .collect::<Vec<_>>()
    );
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".miblo/state.json")).unwrap()).unwrap();
    assert_eq!(state["next_migration"], 2);
    let service = root.join("src/resources/custom_routes/active_users/service.rs");
    fs::write(&service, "pub fn active_users() -> u32 { 42 }\n").unwrap();
    success(cli(
        &[
            "route",
            "add",
            "activate_user",
            "--method",
            "post",
            "--path",
            "/users/{id}/activate",
            "--json-body",
            "--yes",
        ],
        &root,
    ));
    success(cli(&["add", "Group", "title:string", "--yes"], &root));
    success(cli(&["edit", "User", "nickname:string?", "--yes"], &root));
    assert_eq!(
        fs::read_to_string(&service).unwrap(),
        "pub fn active_users() -> u32 { 42 }\n"
    );
    assert!(
        fs::read_to_string(root.join("src/resources/mod.rs"))
            .unwrap()
            .contains("pub mod custom_routes;")
    );
    let before = tree(&root);
    for (name, method, path) in [
        ("duplicate", "GET", "/users/active"),
        ("bad_head", "HEAD", "/users"),
        ("bad_param", "PUT", "/users/{user_id}"),
    ] {
        assert!(
            !cli(
                &[
                    "route", "add", name, "--method", method, "--path", path, "--yes"
                ],
                &root
            )
            .status
            .success()
        );
        assert_eq!(tree(&root), before);
    }
    fs::write(
        root.join("src/resources/custom_routes/mod.rs"),
        "// custom registration\n",
    )
    .unwrap();
    let before = tree(&root);
    failure(
        cli(
            &["route", "add", "extra", "--path", "/extra", "--yes"],
            &root,
        ),
        "registre",
    );
    assert_eq!(tree(&root), before);
}

#[test]
fn route_archives_work_offline_and_refuse_file_collisions() {
    let dir = TempDir::new().unwrap();
    let template = route_pack(dir.path());
    let server = Server::new(archive(&template, false), None);
    let cache = dir.path().join("cache");
    success(cli(
        &[
            "new",
            "sample",
            "--database",
            "sqlite",
            "--template-url",
            &server.url,
            "--template-version",
            "0.5.0",
            "--cache-dir",
            cache.to_str().unwrap(),
            "--yes",
        ],
        dir.path(),
    ));
    drop(server);
    let root = dir.path().join("sample");
    success(cli(
        &[
            "route",
            "add",
            "echo",
            "--path",
            "/echo/{slug}",
            "--method",
            "POST",
            "--json-body",
            "--offline",
            "--cache-dir",
            cache.to_str().unwrap(),
            "--yes",
        ],
        &root,
    ));
    let collision = root.join("src/resources/custom_routes/manual");
    fs::create_dir_all(&collision).unwrap();
    fs::write(collision.join("service.rs"), "// handwritten\n").unwrap();
    let before = tree(&root);
    assert!(
        !cli(
            &[
                "route",
                "add",
                "manual",
                "--path",
                "/manual",
                "--offline",
                "--cache-dir",
                cache.to_str().unwrap(),
                "--yes"
            ],
            &root
        )
        .status
        .success()
    );
    assert_eq!(tree(&root), before);
}

#[test]
fn model_routes_require_capable_templates_and_preserve_models_and_migrations() {
    let dir = TempDir::new().unwrap();
    let root = editable_project(dir.path());
    let old = route_pack(&dir.path().join("old-routes"));
    success(cli(
        &[
            "template",
            "update",
            "--template-dir",
            old.to_str().unwrap(),
        ],
        &root,
    ));
    let args = [
        "route",
        "add",
        "members",
        "--resource",
        "User",
        "--action",
        "create",
        "--path",
        "/members",
        "--yes",
    ];
    let before = tree(&root);
    failure(cli(&args, &root), "pack 0.6.0");
    assert_eq!(tree(&root), before);
    let next = route_pack(&dir.path().join("next-routes"));
    fs::write(next.join("manifest.json"), r#"{"schema_version":3,"id":"rust-axum-sqlite","version":"0.6.0","features":["relations","layered-resources","custom-routes","model-routes"]}"#).unwrap();
    fs::write(next.join("route/mod.rs.hbs"), "pub mod handlers; {{#unless action_handler}}pub mod requests; pub mod service;{{/unless}}\n").unwrap();
    fs::write(next.join("route/handlers.rs.hbs"), "pub use crate::resources::{{resource_module}}::handlers::{{action_handler}} as {{name}};\n").unwrap();
    success(cli(
        &[
            "template",
            "update",
            "--template-dir",
            next.to_str().unwrap(),
        ],
        &root,
    ));
    let before = tree(&root);
    success(cli(&[&args[..], &["--dry-run"]].concat(), &root));
    assert_eq!(tree(&root), before);
    let output = success(cli(&args, &root));
    let message = String::from_utf8(output.stdout).unwrap();
    assert!(message.contains("reliés automatiquement") && !message.contains("501"));
    let alias = root.join("src/resources/custom_routes/members");
    assert!(
        fs::read_to_string(alias.join("handlers.rs"))
            .unwrap()
            .contains("user::handlers::create as members")
    );
    assert!(!alias.join("service.rs").exists() && !alias.join("requests.rs").exists());
    for (path, bytes) in before.iter().filter(|(p, _)| {
        p.starts_with(root.join("src/resources/user")) || p.starts_with(root.join("migrations"))
    }) {
        assert_eq!(&fs::read(path).unwrap(), bytes);
    }
    let config = fs::read_to_string(root.join("miblo.yaml")).unwrap();
    assert!(config.contains("action: create") && config.contains("method: POST"));
    let before = tree(&root);
    for (method, action, path) in [
        ("GET", "create", "/other"),
        ("GET", "get", "/other"),
        ("GET", "list", "/other/{id}"),
        ("PATCH", "update", "/other/{id}/{group_id}"),
    ] {
        let output = cli(
            &[
                "route",
                "add",
                "invalid",
                "--resource",
                "User",
                "--action",
                action,
                "--method",
                method,
                "--path",
                path,
                "--yes",
            ],
            &root,
        );
        assert!(!output.status.success());
        assert_eq!(tree(&root), before);
    }
    failure(
        cli(
            &[
                "template",
                "update",
                "--template-dir",
                old.to_str().unwrap(),
            ],
            &root,
        ),
        "pack 0.6.0",
    );
    assert_eq!(tree(&root), before);
    success(cli(&["edit", "User", "alias:string?", "--yes"], &root));
    assert!(
        fs::read_to_string(alias.join("handlers.rs"))
            .unwrap()
            .contains("user::handlers::create as members")
    );
    success(cli(&["doctor"], &root));
}
