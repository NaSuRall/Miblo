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
        "Template changed",
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
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut byte = [0; 1];
                        while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                            match stream.read(&mut byte) {
                                Ok(1) => request.push(byte[0]),
                                _ => break,
                            }
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
