use anyhow::{Context, Result, bail, ensure};
use flate2::read::GzDecoder;
use handlebars::Handlebars;
use reqwest::{Url, blocking::Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};

pub const DEFAULT_BASE: &str = "https://github.com/NaSuRall/miblo_templates/releases/download";
pub const PROJECT_FILES: &[(&str, &str)] = &[
    ("project/Cargo.toml.hbs", "Cargo.toml"),
    ("project/Cargo.lock.hbs", "Cargo.lock"),
    ("project/main.rs.hbs", "src/main.rs"),
    ("project/lib.rs.hbs", "src/lib.rs"),
    ("project/error.rs.hbs", "src/error.rs"),
    ("project/patch.rs.hbs", "src/patch.rs"),
    ("project/build.rs.hbs", "build.rs"),
    ("project/env.hbs", ".env.example"),
    ("project/gitignore.hbs", ".gitignore"),
    ("project/README.md.hbs", "README.md"),
    ("project/compose.yaml.hbs", "compose.yaml"),
    ("project/health.rs.hbs", "tests/health.rs"),
];
pub const RESOURCE_FILES: &[(&str, &str)] = &[
    ("resource/mod.rs.hbs", "mod.rs"),
    ("resource/model.rs.hbs", "model.rs"),
    ("resource/handlers.rs.hbs", "handlers.rs"),
];
const LAYERED_PROJECT_FILES: &[(&str, &str)] = &[
    ("project/config.rs.hbs", "src/config.rs"),
    ("project/database.rs.hbs", "src/database.rs"),
];
const LAYERED_RESOURCE_FILES: &[(&str, &str)] = &[
    ("resource/requests.rs.hbs", "requests.rs"),
    ("resource/service.rs.hbs", "service.rs"),
    ("resource/repository.rs.hbs", "repository.rs"),
];
pub const ROUTE_FILES: &[(&str, &str)] = &[
    ("route/mod.rs.hbs", "mod.rs"),
    ("route/handlers.rs.hbs", "handlers.rs"),
    ("route/requests.rs.hbs", "requests.rs"),
    ("route/service.rs.hbs", "service.rs"),
];
const MAX_ARCHIVE: u64 = 4 * 1024 * 1024;
const MAX_EXPANDED: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateLock {
    pub version: String,
    pub fingerprint: String,
    pub local_path: Option<PathBuf>,
    pub url: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    id: String,
    version: String,
    #[serde(default)]
    features: Vec<String>,
}

pub struct Pack {
    pub format: u32,
    pub lock: TemplateLock,
    pub id: String,
    pub features: Vec<String>,
    registry: Handlebars<'static>,
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Suggest a checked-out pack to the interactive UI, without changing CLI defaults.
/// Only inspect known pack locations alongside the current directory and its parents.
pub fn discover_local(start: &Path, database: crate::config::Database) -> Option<PathBuf> {
    let folders = if database == crate::config::Database::Sqlite {
        [
            "rust_sqlite_template_v3",
            "rust_sqlite_template_v2",
            "rust_sqlite_template_v1",
        ]
    } else {
        ["rust_template_v3", "rust_template_v2", "rust_template_v1"]
    };
    for parent in start.ancestors() {
        for folder in folders {
            for relative in [format!("miblo_templates/{folder}"), folder.into()] {
                let candidate = parent.join(relative);
                if let Ok(pack) = Pack::local(&candidate)
                    && pack.id == database.stack()
                {
                    return pack.lock.local_path;
                }
            }
        }
    }
    None
}

fn names(format: u32) -> Vec<&'static str> {
    let mut names: Vec<_> = PROJECT_FILES
        .iter()
        .chain(RESOURCE_FILES)
        .map(|(name, _)| *name)
        .collect();
    if format >= 2 {
        names.extend(
            LAYERED_PROJECT_FILES
                .iter()
                .chain(LAYERED_RESOURCE_FILES)
                .map(|(name, _)| *name),
        );
    }
    if format >= 3 {
        names.extend(ROUTE_FILES.iter().map(|(name, _)| *name));
        names.extend(["route/registry.rs.hbs", "route/error.rs.hbs"]);
    }
    names.extend([
        "registry.rs.hbs",
        "resource/migration.sql.hbs",
        "manifest.json",
    ]);
    names
}

impl Pack {
    pub fn project_files(&self) -> Vec<(&'static str, &'static str)> {
        let mut files = PROJECT_FILES.to_vec();
        if self.format >= 2 {
            files.extend_from_slice(LAYERED_PROJECT_FILES);
        }
        files
    }
    pub fn resource_files(&self) -> Vec<(&'static str, &'static str)> {
        let mut files = RESOURCE_FILES.to_vec();
        if self.format >= 2 {
            files.extend_from_slice(LAYERED_RESOURCE_FILES);
        }
        files
    }
    pub fn local(path: &Path) -> Result<Self> {
        let path = path
            .canonicalize()
            .context("Template directory not found")?;
        let mut files = BTreeMap::new();
        ensure!(
            path.join("manifest.json")
                .symlink_metadata()?
                .file_type()
                .is_file(),
            "Template manifest must be a regular file"
        );
        let manifest: Manifest = serde_json::from_slice(&read_limited(
            fs::File::open(path.join("manifest.json"))?,
            MAX_ARCHIVE,
        )?)?;
        for name in names(manifest.schema_version) {
            let file = path.join(name);
            ensure!(
                file.symlink_metadata()?.file_type().is_file(),
                "Template must be a regular file: {name}"
            );
            ensure!(
                file.canonicalize()?.starts_with(&path),
                "Template leaves its directory: {name}"
            );
            let bytes = read_limited(fs::File::open(&file)?, MAX_ARCHIVE)?;
            files.insert(name.into(), String::from_utf8(bytes)?);
        }
        Self::from_files(
            files,
            TemplateLock {
                version: String::new(),
                fingerprint: String::new(),
                local_path: Some(path),
                url: None,
                sha256: None,
            },
        )
    }

    pub fn remote(
        url: &str,
        version: &str,
        expected: Option<&str>,
        offline: bool,
        cache: &Path,
    ) -> Result<Self> {
        validate_url(url)?;
        let mut pin = expected.map(str::to_owned);
        // An index remembers the immutable digest for a URL/version after its first download.
        let index = cache.join(format!(
            "{}.json",
            digest(format!("{url}\n{version}").as_bytes())
        ));
        if pin.is_none() && index.is_file() {
            let cached: TemplateLock = serde_json::from_slice(&fs::read(&index)?)?;
            ensure!(
                cached.url.as_deref() == Some(url) && cached.version == version,
                "Invalid template cache index"
            );
            pin = cached.sha256;
        }
        let sha = match pin {
            Some(sha) => sha,
            None => {
                ensure!(
                    !offline,
                    "Template is not cached; download it once without --offline or use --template-dir"
                );
                let checksum = download(&format!("{url}.sha256"), 1024)?;
                String::from_utf8(checksum)?
                    .split_whitespace()
                    .next()
                    .context("Empty template checksum")?
                    .to_owned()
            }
        };
        ensure!(
            sha.len() == 64 && sha.bytes().all(|c| c.is_ascii_hexdigit()),
            "Invalid SHA-256 checksum"
        );
        let sha = sha.to_lowercase();
        let archive_path = cache.join(format!("{sha}.tar.gz"));
        let bytes = if archive_path.is_file() {
            read_limited(fs::File::open(&archive_path)?, MAX_ARCHIVE)?
        } else {
            ensure!(
                !offline,
                "Pinned template is not cached; retry without --offline"
            );
            download(url, MAX_ARCHIVE)?
        };
        ensure!(
            digest(&bytes) == sha,
            "Template SHA-256 mismatch; no project files were written"
        );
        let lock = TemplateLock {
            version: version.into(),
            fingerprint: String::new(),
            local_path: None,
            url: Some(url.into()),
            sha256: Some(sha),
        };
        let pack = Self::from_files(unpack(&bytes)?, lock)?;
        ensure!(
            pack.lock.version == version,
            "Template version does not match requested version {version}"
        );
        fs::create_dir_all(cache).context("Cannot create template cache; use --cache-dir")?;
        if !archive_path.exists() {
            cache_write(&archive_path, &bytes)?;
        }
        if !index.exists() {
            cache_write(&index, &serde_json::to_vec_pretty(&pack.lock)?)?;
        }
        Ok(pack)
    }

    pub fn pinned(lock: &TemplateLock, offline: bool, cache: &Path) -> Result<Self> {
        let pack = match (&lock.local_path, &lock.url) {
            (Some(path), None) => Self::local(path)?,
            (None, Some(url)) => {
                Self::remote(url, &lock.version, lock.sha256.as_deref(), offline, cache)?
            }
            _ => bail!("Invalid template source in .miblo/state.json"),
        };
        ensure!(
            pack.lock.fingerprint == lock.fingerprint && pack.lock.version == lock.version,
            "Template changed since project creation; restore the pinned template before adding a resource"
        );
        Ok(pack)
    }

    fn from_files(files: BTreeMap<String, String>, mut lock: TemplateLock) -> Result<Self> {
        let manifest: Manifest = serde_json::from_str(
            files
                .get("manifest.json")
                .context("Missing template manifest")?,
        )?;
        for name in names(manifest.schema_version) {
            ensure!(files.contains_key(name), "Missing template file: {name}");
        }
        ensure!(
            matches!(manifest.schema_version, 1..=3)
                && matches!(manifest.id.as_str(), "rust-axum-mysql" | "rust-axum-sqlite"),
            "Unsupported template pack format or stack"
        );
        ensure!(
            files
                .keys()
                .all(|name| names(manifest.schema_version).contains(&name.as_str())),
            "Unexpected file for this template format"
        );
        if !lock.version.is_empty() {
            ensure!(
                lock.version == manifest.version,
                "Wrong template pack version"
            );
        }
        lock.version = manifest.version;
        lock.fingerprint = digest(&serde_json::to_vec(&files)?);
        let mut registry = Handlebars::new();
        registry.set_strict_mode(true);
        registry.register_escape_fn(handlebars::no_escape);
        for (name, text) in files {
            if name.ends_with(".hbs") {
                registry
                    .register_template_string(&name, text)
                    .with_context(|| format!("Invalid template {name}"))?;
            }
        }
        Ok(Self {
            format: manifest.schema_version,
            lock,
            registry,
            id: manifest.id,
            features: manifest.features,
        })
    }

    pub fn render(&self, name: &str, data: &serde_json::Value) -> Result<String> {
        self.registry
            .render(name, data)
            .with_context(|| format!("Cannot render {name}"))
    }
}

fn cache_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().context("Invalid cache path")?)?;
    file.write_all(bytes)?;
    match file.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn validate_url(url: &str) -> Result<()> {
    let url = Url::parse(url)?;
    ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Template URLs cannot contain credentials, queries or fragments"
    );
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    ensure!(
        url.scheme() == "https" || (url.scheme() == "http" && loopback),
        "Use HTTPS for templates (HTTP is allowed only on loopback for local development)"
    );
    Ok(())
}

fn download(url: &str, limit: u64) -> Result<Vec<u8>> {
    validate_url(url)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .connect_timeout(Duration::from_secs(10))
        .user_agent(concat!("miblo/", env!("CARGO_PKG_VERSION")))
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("Too many redirects");
            }
            // GitHub release assets redirect to a signed HTTPS URL.
            if attempt.url().scheme() == "https" {
                attempt.follow()
            } else {
                attempt.error("Refusing non-HTTPS redirect")
            }
        }))
        .build()?;
    eprintln!("Downloading template asset...");
    let response = client
        .get(url)
        .send()
        .context("Cannot download templates; check the release URL or use --template-dir")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        bail!(
            "Template asset unavailable (HTTP 404): publish the archive and its .sha256 file.\n\
             Dans Miblo : choisis Dossier local ou une autre URL de template.\n\
             En ligne de commande : utilise --template-dir <dossier>.\n\
             URL: {url}"
        );
    }
    let response = response
        .error_for_status()
        .context("Template download failed; check the server or use --template-dir")?;
    ensure!(
        response.content_length().is_none_or(|size| size <= limit),
        "Template download is too large"
    );
    read_limited(response, limit)
}

fn read_limited(reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= limit, "Template size limit exceeded");
    Ok(bytes)
}

fn unpack(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    let decoder = GzDecoder::new(bytes);
    // Bound the entire decompressed stream, including tar metadata.
    let expanded = read_limited(decoder, MAX_EXPANDED)?;
    let mut archive = tar::Archive::new(expanded.as_slice());
    let mut files = BTreeMap::new();
    for (index, entry) in archive.entries()?.enumerate() {
        ensure!(index < 128, "Too many files in template archive");
        let entry = entry?;
        let path = entry.path()?.into_owned();
        ensure!(
            path.components()
                .all(|part| matches!(part, Component::Normal(_)))
                && !path.as_os_str().is_empty(),
            "Unsafe archive path"
        );
        if entry.header().entry_type().is_dir() {
            continue;
        }
        ensure!(
            entry.header().entry_type().is_file(),
            "Template archive cannot contain links or special files"
        );
        let name = path.to_str().context("Non-UTF8 template path")?.to_owned();
        ensure!(
            names(3).contains(&name.as_str()),
            "Unexpected template file: {name}"
        );
        ensure!(
            !files.contains_key(&name),
            "Duplicate archive entry: {name}"
        );
        let content = String::from_utf8(read_limited(entry, MAX_ARCHIVE)?)?;
        files.insert(name, content);
    }
    Ok(files)
}

pub fn default_cache() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("miblo")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_only_valid_neighbor_packs_from_nested_directories() {
        let dir = tempfile::TempDir::new().unwrap();
        let start = dir.path().join("Miblo/src");
        fs::create_dir_all(&start).unwrap();
        let invalid = dir.path().join("Miblo/rust_template_v1");
        fs::create_dir_all(&invalid).unwrap();
        fs::write(invalid.join("manifest.json"), "{}").unwrap();
        assert!(discover_local(&start, crate::config::Database::Mysql).is_none());

        let pack = dir.path().join("miblo_templates/rust_template_v1");
        for name in names(1) {
            let file = pack.join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            let content = if name == "manifest.json" {
                r#"{"schema_version":1,"id":"rust-axum-mysql","version":"0.1.0"}"#
            } else {
                "{{name}}"
            };
            fs::write(file, content).unwrap();
        }
        assert_eq!(
            discover_local(&start, crate::config::Database::Mysql),
            Some(pack.canonicalize().unwrap())
        );
        fs::remove_file(pack.join("registry.rs.hbs")).unwrap();
        assert!(discover_local(&start, crate::config::Database::Mysql).is_none());
    }
}
