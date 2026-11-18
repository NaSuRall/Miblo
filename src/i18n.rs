//! Application language and per-user preferences. Project files and HTTP data
//! are never translated. Workers read the current language; CLI children receive
//! it explicitly so changing it does not mutate the process environment.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU8, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Fr,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "en" => Ok(Self::En),
            "fr" => Ok(Self::Fr),
            _ => bail!("MIBLO_LANGUAGE must be en or fr"),
        }
    }

    pub fn text(self, english: &'static str, french: &'static str) -> &'static str {
        match self {
            Self::En => english,
            Self::Fr => french,
        }
    }
}

static LANGUAGE: AtomicU8 = AtomicU8::new(0);

pub fn current() -> Language {
    if LANGUAGE.load(Ordering::Relaxed) == 1 {
        Language::Fr
    } else {
        Language::En
    }
}

pub fn set(language: Language) {
    LANGUAGE.store(u8::from(language == Language::Fr), Ordering::Relaxed);
}

// Both translations live at their call site and are checked by the compiler.
// Format strings stay literals: interpolation never rewrites user-provided text.
macro_rules! tr {
    ($en:literal, $fr:literal) => {
        $crate::i18n::current().text($en, $fr)
    };
}

macro_rules! trf {
    ($en:literal, $fr:literal $($args:tt)*) => {
        match $crate::i18n::current() {
            $crate::i18n::Language::En => format!($en $($args)*),
            $crate::i18n::Language::Fr => format!($fr $($args)*),
        }
    };
}

#[derive(Default, Serialize, Deserialize)]
struct Preferences {
    #[serde(default)]
    language: Language,
    // Preserve settings introduced by other versions when updating the language.
    #[serde(flatten)]
    other: BTreeMap<String, serde_json::Value>,
}

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn discover() -> Result<Self> {
        fn nonempty(name: &str) -> Option<PathBuf> {
            env::var_os(name)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
        }
        let directory = if let Some(path) = nonempty("MIBLO_CONFIG_DIR") {
            path
        } else if cfg!(windows) {
            nonempty("APPDATA")
                .context("APPDATA is unavailable; set MIBLO_CONFIG_DIR")?
                .join("miblo")
        } else if let Some(path) = nonempty("XDG_CONFIG_HOME") {
            path.join("miblo")
        } else {
            let home = nonempty("HOME").context("HOME is unavailable; set MIBLO_CONFIG_DIR")?;
            if cfg!(target_os = "macos") {
                home.join("Library/Application Support/miblo")
            } else {
                home.join(".config/miblo")
            }
        };
        Ok(Self {
            path: directory.join("settings.json"),
        })
    }

    fn read(&self) -> Result<Preferences> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .with_context(|| format!("Invalid preferences: {}", self.path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Preferences::default())
            }
            Err(error) => Err(error)
                .with_context(|| format!("Cannot read preferences: {}", self.path.display())),
        }
    }

    pub fn save(&self, language: Language) -> Result<()> {
        let mut preferences = self.read()?;
        preferences.language = language;
        let parent = self
            .path
            .parent()
            .context("Invalid preferences directory")?;
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut file, &preferences)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(&self.path)
            .with_context(|| format!("Cannot save preferences: {}", self.path.display()))?;
        Ok(())
    }
}

/// Missing preferences mean English, regardless of the operating system locale.
/// A broken settings file must not prevent the CLI or workspace from starting.
pub fn initialize() -> Option<String> {
    let loaded = Store::discover().and_then(|store| store.read());
    let (saved, mut warning) = match loaded {
        Ok(preferences) => (preferences.language, None),
        Err(error) => (Language::En, Some(format!("{error:#}"))),
    };
    let language = match env::var("MIBLO_LANGUAGE") {
        Ok(value) => match Language::parse(&value) {
            Ok(language) => language,
            Err(error) => {
                warning = Some(error.to_string());
                saved
            }
        },
        Err(_) => saved,
    };
    set(language);
    warning
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_install_is_english_and_preferences_round_trip_without_losing_other_settings() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store {
            path: directory.path().join("miblo/settings.json"),
        };
        assert_eq!(store.read().unwrap().language, Language::En);
        assert!(!store.path.exists());
        store.save(Language::Fr).unwrap();
        assert_eq!(store.read().unwrap().language, Language::Fr);
        fs::write(&store.path, r#"{"language":"fr","theme":"future-theme"}"#).unwrap();
        store.save(Language::En).unwrap();
        let saved = store.read().unwrap();
        assert_eq!(saved.language, Language::En);
        assert_eq!(saved.other["theme"], "future-theme");
    }

    #[test]
    fn invalid_preferences_are_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store {
            path: directory.path().join("settings.json"),
        };
        let broken = "{invalid-json";
        fs::write(&store.path, broken).unwrap();
        assert!(store.save(Language::Fr).is_err());
        assert_eq!(fs::read_to_string(&store.path).unwrap(), broken);
        assert!(Language::parse("de").is_err());
        assert_eq!(Language::parse("FR").unwrap(), Language::Fr);
    }
}
