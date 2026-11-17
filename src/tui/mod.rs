mod forms;
mod process;
mod view;

use crate::{
    config::{Database, Project},
    database::{self, Action},
    generation, wizard,
};
use anyhow::{Context, Result, ensure};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use process::Process;
use std::{
    collections::{HashSet, VecDeque},
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver, SyncSender},
    time::Duration,
};

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Files,
    Code,
    Logs,
    Resources,
}
#[derive(Clone)]
struct Entry {
    path: PathBuf,
    label: String,
    dir: bool,
}

struct App {
    root: Option<PathBuf>,
    project: Option<Project>,
    entries: Vec<Entry>,
    expanded: HashSet<PathBuf>,
    selected: usize,
    resource_selected: usize,
    focus: Focus,
    file: Option<PathBuf>,
    code: Vec<String>,
    code_scroll: usize,
    code_horizontal: u16,
    log_scroll: usize,
    logs: VecDeque<String>,
    status: String,
    notice: String,
    theme: usize,
    dialog: Option<forms::Dialog>,
    new_draft: forms::NewDraft,
    add_draft: forms::AddDraft,
    api: Option<Process>,
    api_status: String,
    job: Option<Process>,
    job_destination: Option<PathBuf>,
    sender: SyncSender<String>,
    receiver: Receiver<String>,
    quit: bool,
    editor_requested: bool,
    db_draft: database::MysqlSettings,
    db_status: String,
    db_job: Option<Action>,
    migrate_after_add: bool,
}

pub fn run(project: Option<PathBuf>) -> Result<()> {
    ensure!(
        wizard::interactive(),
        "The workspace requires a terminal; use miblo --help for scriptable commands"
    );
    let mut app = App::new();
    if let Some(path) = project {
        app.open_project(&path)?;
    } else if Path::new("miblo.yaml").is_file() {
        app.open_project(Path::new("."))?;
    }
    ratatui::run(|terminal| -> Result<()> {
        while !app.quit {
            app.tick();
            terminal.draw(|frame| view::draw(frame, &mut app))?;
            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind != KeyEventKind::Release
                && let Err(error) = app.key(key)
            {
                app.report(format!("{error:#}"));
            }
            if app.editor_requested {
                app.editor_requested = false;
                // Restore cooked mode and the original screen before handing off.
                ratatui::restore();
                let result = app.open_editor();
                *terminal = ratatui::try_init()?;
                if let Err(error) = result {
                    app.report(format!("{error:#}"));
                }
                if let Err(error) = app.refresh() {
                    app.report(format!("{error:#}"));
                }
            }
        }
        Ok(())
    })
}

impl App {
    fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(256);
        Self {
            root: None,
            project: None,
            entries: Vec::new(),
            expanded: HashSet::new(),
            selected: 0,
            resource_selected: 0,
            focus: Focus::Files,
            file: None,
            code: Vec::new(),
            code_scroll: 0,
            code_horizontal: 0,
            log_scroll: 0,
            logs: VecDeque::new(),
            status: "Aucun projet ouvert".into(),
            notice: "n : créer une API • o : ouvrir un projet • Espace : actions • ? : aide".into(),
            theme: 0,
            dialog: None,
            new_draft: forms::NewDraft::default(),
            add_draft: forms::AddDraft::default(),
            api: None,
            api_status: "Arrêtée".into(),
            job: None,
            job_destination: None,
            sender,
            receiver,
            quit: false,
            editor_requested: false,
            db_draft: database::MysqlSettings::default(),
            db_status: "Non vérifiée".into(),
            db_job: None,
            migrate_after_add: false,
        }
    }

    fn open_project(&mut self, path: &Path) -> Result<()> {
        ensure!(
            self.api.is_none() && self.job.is_none(),
            "Arrête l’API et attends la fin de l’action avant de changer de projet"
        );
        let root = path.canonicalize().context("Dossier introuvable")?;
        let project = Project::load(&root.join("miblo.yaml"))?;
        self.root = Some(root);
        self.project = Some(project);
        self.db_status = "Non vérifiée".into();
        self.selected = 0;
        self.resource_selected = 0;
        self.expanded = ["src", "src/resources", "migrations"]
            .into_iter()
            .map(PathBuf::from)
            .collect();
        self.file = None;
        self.code.clear();
        self.code_scroll = 0;
        self.refresh()?;
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.path == Path::new("src/main.rs"))
        {
            self.selected = index;
            self.open_selected()?;
        }
        self.notice = "Projet ouvert • Tab : changer de panneau • Espace : actions".into();
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        let Some(root) = self.root.clone() else {
            return Ok(());
        };
        self.project = Some(Project::load(&root.join("miblo.yaml"))?);
        self.status = match generation::inspect(&root) {
            Ok(_) => "Configuration cohérente".into(),
            Err(e) => format!("À vérifier : {e}"),
        };
        let selected = self
            .entries
            .get(self.selected)
            .map(|entry| entry.path.clone());
        self.entries = file_tree(&root, &self.expanded)?;
        self.selected = selected
            .and_then(|path| self.entries.iter().position(|entry| entry.path == path))
            .unwrap_or(self.selected)
            .min(self.entries.len().saturating_sub(1));
        self.resource_selected = self.resource_selected.min(
            self.project
                .as_ref()
                .map_or(0, |p| p.resources.len().saturating_sub(1)),
        );
        if let Some(path) = self.file.clone() {
            self.load_file(&path)?;
        }
        Ok(())
    }

    fn load_file(&mut self, relative: &Path) -> Result<()> {
        let root = self.root.as_ref().context("Ouvre un projet d’abord")?;
        let path = root.join(relative).canonicalize()?;
        ensure!(
            path.starts_with(root) && path.is_file(),
            "Le fichier doit appartenir au projet"
        );
        ensure!(
            !secret_file(relative),
            "Les fichiers .env privés ne sont pas affichés"
        );
        let mut bytes = Vec::new();
        fs::File::open(&path)?
            .take(128 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        let truncated = bytes.len() > 128 * 1024;
        bytes.truncate(128 * 1024);
        ensure!(!bytes.contains(&0), "Ce fichier est binaire");
        self.code = String::from_utf8_lossy(&bytes).lines().map(clean).collect();
        if truncated {
            self.code.push(
                "… aperçu limité à 128 Kio — ouvre le fichier dans ton éditeur pour la suite"
                    .into(),
            );
        }
        self.file = Some(relative.into());
        self.code_scroll = self.code_scroll.min(self.code.len().saturating_sub(1));
        Ok(())
    }

    fn open_selected(&mut self) -> Result<()> {
        if self.focus == Focus::Resources {
            let resource = self
                .project
                .as_ref()
                .and_then(|p| p.resources.get(self.resource_selected))
                .context("Aucune ressource")?;
            let path = PathBuf::from(format!("src/resources/{}/handlers.rs", resource.module()));
            if let Some(parent) = path.parent() {
                self.expanded.insert(parent.to_path_buf());
            }
            self.refresh()?;
            if let Some(index) = self.entries.iter().position(|entry| entry.path == path) {
                self.selected = index;
            }
            self.load_file(&path)?;
            self.focus = Focus::Code;
        } else if let Some(entry) = self.entries.get(self.selected).cloned() {
            if entry.dir {
                if !self.expanded.remove(&entry.path) {
                    self.expanded.insert(entry.path);
                }
                self.refresh()?;
            } else {
                self.code_scroll = 0;
                self.code_horizontal = 0;
                self.load_file(&entry.path)?;
            }
        }
        Ok(())
    }

    fn key(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.dialog.is_some() {
                self.dialog = None;
                self.db_draft.password.clear();
                self.notice = "Action annulée".into();
            } else {
                self.request_quit()?;
            }
            return Ok(());
        }
        if self.dialog.is_some() {
            return self.dialog_key(key);
        }
        match key.code {
            KeyCode::Char(':') => self.input(forms::InputKind::Command, "Commande", ""),
            KeyCode::Char(' ') => self.palette(),
            KeyCode::Char('?') | KeyCode::F(1) => self.dialog = Some(forms::Dialog::Help),
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Files => Focus::Code,
                    Focus::Code => Focus::Logs,
                    Focus::Logs => Focus::Resources,
                    Focus::Resources => Focus::Files,
                }
            }
            KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Files => Focus::Resources,
                    Focus::Resources => Focus::Logs,
                    Focus::Logs => Focus::Code,
                    Focus::Code => Focus::Files,
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(12),
            KeyCode::PageUp => self.move_selection(-12),
            KeyCode::Right | KeyCode::Char('l') if self.focus == Focus::Code => {
                self.code_horizontal = self.code_horizontal.saturating_add(4)
            }
            KeyCode::Left | KeyCode::Char('h') if self.focus == Focus::Code => {
                self.code_horizontal = self.code_horizontal.saturating_sub(4)
            }
            KeyCode::Enter | KeyCode::Right
                if matches!(self.focus, Focus::Files | Focus::Resources) =>
            {
                self.open_selected()?
            }
            KeyCode::Char('n') => self.start_new()?,
            KeyCode::Char('a') => self.start_add()?,
            KeyCode::Char('b') => self.database_menu()?,
            KeyCode::Char('o') => self.input(forms::InputKind::Open, "Dossier du projet", "."),
            KeyCode::Char('r') => self.run_api()?,
            KeyCode::Char('s') => self.stop_api(),
            KeyCode::Char('c') => self.cargo_job("check")?,
            KeyCode::Char('e') => self.request_editor()?,
            KeyCode::Char('t') => {
                self.theme = (self.theme + 1) % 3;
                self.notice = "Thème modifié pour cette session".into();
            }
            KeyCode::Char('q') => self.request_quit()?,
            KeyCode::F(5) => self.refresh()?,
            _ => {}
        }
        Ok(())
    }

    fn move_selection(&mut self, delta: isize) {
        let (value, maximum) = match self.focus {
            Focus::Files => (&mut self.selected, self.entries.len()),
            Focus::Resources => (
                &mut self.resource_selected,
                self.project.as_ref().map_or(0, |p| p.resources.len()),
            ),
            Focus::Code => (&mut self.code_scroll, self.code.len()),
            Focus::Logs => {
                self.log_scroll = self
                    .log_scroll
                    .saturating_add_signed(-delta)
                    .min(self.logs.len().saturating_sub(1));
                return;
            }
        };
        *value = value
            .saturating_add_signed(delta)
            .min(maximum.saturating_sub(1));
    }

    fn report(&mut self, message: String) {
        self.notice = clean(&message);
        self.log(message);
    }
    fn log(&mut self, message: String) {
        if self.logs.len() >= 1000 {
            self.logs.pop_front();
        }
        self.logs.push_back(clean(&message));
    }
    fn tick(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            if message.starts_with("[API]")
                && message.contains("Server running on")
                && self.api.is_some()
            {
                self.api_status = "En cours".into();
                self.db_status = "Migrations à jour".into();
            }
            self.log(message);
        }
        let finished = self
            .api
            .as_mut()
            .and_then(|process| process.poll().ok().flatten());
        if let Some(status) = finished {
            self.api = None;
            self.api_status = format!("Terminée ({status})");
            self.log(format!("API : {status}"));
        }
        let finished = self
            .job
            .as_mut()
            .and_then(|process| process.poll().ok().flatten());
        if let Some(status) = finished {
            let label = self.job.take().map(|p| p.label.clone()).unwrap_or_default();
            let db_action = self.db_job.take();
            let migrate_after_add = std::mem::take(&mut self.migrate_after_add);
            self.notice = format!("{label} : {status}");
            self.log(self.notice.clone());
            if status.success() {
                if migrate_after_add {
                    self.db_status = "Migrations en attente".into();
                }
                if let Some(action) = db_action {
                    self.db_status = if action == Action::Check {
                        "Connexion vérifiée"
                    } else {
                        "Migrations à jour"
                    }
                    .into();
                }
                if let Some(root) = self.job_destination.take() {
                    if let Err(error) = self.open_project(&root) {
                        self.report(format!("{error:#}"));
                    } else if self
                        .project
                        .as_ref()
                        .is_some_and(|p| p.database() == Database::Sqlite)
                    {
                        if let Err(error) = self.database_action(Action::Init) {
                            self.report(format!("{error:#}"));
                        }
                    } else if let Err(error) = self.start_database_setup() {
                        self.report(format!("{error:#}"));
                    }
                } else if let Err(error) = self.refresh() {
                    self.report(format!("{error:#}"));
                } else if migrate_after_add
                    && self.api.is_none()
                    && self
                        .project
                        .as_ref()
                        .is_some_and(|p| p.database() == Database::Sqlite)
                    && let Err(error) = self.database_action(Action::Migrate)
                {
                    self.report(format!("{error:#}"));
                }
            } else if self.job_destination.take().is_some() {
                self.creation_failed();
            } else if db_action.is_some() {
                self.db_status = "Échec • consulter les logs".into();
            }
        }
    }

    fn ready_for_job(&self) -> Result<&Path> {
        ensure!(self.job.is_none(), "Une action est déjà en cours");
        self.root
            .as_deref()
            .context("Aucun projet ouvert : n pour créer une API, o pour ouvrir un projet")
    }
    fn run_api(&mut self) -> Result<()> {
        ensure!(self.api.is_none(), "L’API est déjà lancée par Miblo");
        let root = self.ready_for_job()?;
        let url = match database::connection_url(root) {
            Ok(url) => url,
            Err(error) => {
                self.report(error.to_string());
                self.start_database_setup()?;
                return Ok(());
            }
        };
        let mut command = Command::new("cargo");
        command
            .arg("run")
            .env("DATABASE_URL", url)
            .current_dir(root);
        self.api = Some(Process::spawn(command, "API", self.sender.clone())?);
        self.api_status = "Compilation / démarrage".into();
        self.log("Lancement de l’API…".into());
        Ok(())
    }
    fn stop_api(&mut self) {
        self.api = None;
        self.api_status = "Arrêtée".into();
        self.log("API arrêtée".into());
    }
    fn cargo_job(&mut self, action: &str) -> Result<()> {
        let root = self.ready_for_job()?;
        let mut command = Command::new("cargo");
        command.arg(action).current_dir(root);
        self.job = Some(Process::spawn(
            command,
            &format!("cargo {action}"),
            self.sender.clone(),
        )?);
        Ok(())
    }
    fn cli_job(
        &mut self,
        args: Vec<OsString>,
        label: &str,
        destination: Option<PathBuf>,
        remove_local_env: bool,
    ) -> Result<()> {
        ensure!(self.job.is_none(), "Une action est déjà en cours");
        let mut command = Command::new(std::env::current_exe()?);
        command.args(args);
        if remove_local_env {
            command.env_remove("MIBLO_TEMPLATE_DIR");
        }
        self.job = Some(Process::spawn(command, label, self.sender.clone())?);
        self.job_destination = destination;
        self.dialog = None;
        self.notice = format!("{label} en cours…");
        Ok(())
    }

    fn request_editor(&mut self) -> Result<()> {
        ensure!(
            self.job.is_none(),
            "Attends la fin de la génération avant d’ouvrir l’éditeur"
        );
        ensure!(self.file.is_some(), "Sélectionne un fichier d’abord");
        self.editor_requested = true;
        Ok(())
    }
    fn open_editor(&self) -> Result<()> {
        let root = self.root.as_ref().context("Aucun projet")?;
        let path = root
            .join(self.file.as_ref().context("Aucun fichier")?)
            .canonicalize()?;
        ensure!(
            path.starts_with(root),
            "Le fichier doit appartenir au projet"
        );
        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| {
                if cfg!(windows) {
                    "notepad".into()
                } else {
                    "vi".into()
                }
            });
        let words = shell_words::split(&editor).context("VISUAL/EDITOR est invalide")?;
        let executable = words.first().context("VISUAL/EDITOR est vide")?;
        let status = Command::new(executable)
            .args(&words[1..])
            .arg(path)
            .current_dir(root)
            .status()?;
        ensure!(status.success(), "L’éditeur s’est terminé avec {status}");
        Ok(())
    }

    fn request_quit(&mut self) -> Result<()> {
        ensure!(
            self.job.is_none(),
            "Une action est en cours ; attends sa fin pour quitter sans interrompre l’écriture des fichiers"
        );
        self.quit = true;
        Ok(())
    }

    fn command(&mut self, value: &str) -> Result<()> {
        let (name, argument) = value.trim().split_once(' ').unwrap_or((value.trim(), ""));
        match name {
            "q" | "quit" => self.request_quit()?,
            "new" => self.start_new()?,
            "add" => self.start_add()?,
            "db" | "database" => self.database_menu()?,
            "migrate" => self.confirm_database_action(Action::Migrate)?,
            "open" if argument.is_empty() => {
                self.input(forms::InputKind::Open, "Dossier du projet", ".")
            }
            "open" => self.open_project(Path::new(argument.trim()))?,
            "run" | "start" => self.run_api()?,
            "stop" => self.stop_api(),
            "check" => self.cargo_job("check")?,
            "test" => self.cargo_job("test")?,
            "doctor" => {
                let root = self.ready_for_job()?.as_os_str().to_owned();
                self.cli_job(
                    vec!["doctor".into(), "--project".into(), root],
                    "Diagnostic",
                    None,
                    false,
                )?;
            }
            "edit" | "e" => self.request_editor()?,
            "refresh" => self.refresh()?,
            "theme" => self.theme = (self.theme + 1) % 3,
            "help" => self.dialog = Some(forms::Dialog::Help),
            _ => anyhow::bail!("Commande inconnue. :help affiche les commandes disponibles."),
        }
        Ok(())
    }
}

fn clean(value: &str) -> String {
    value
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .filter(|c| !c.is_control())
        .collect()
}
fn secret_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|v| v.to_str())
        .is_some_and(|name| (name == ".env" || name.starts_with(".env.")) && name != ".env.example")
}
fn file_tree(root: &Path, expanded: &HashSet<PathBuf>) -> Result<Vec<Entry>> {
    fn visit(
        root: &Path,
        relative: &Path,
        expanded: &HashSet<PathBuf>,
        entries: &mut Vec<Entry>,
        depth: usize,
    ) -> Result<()> {
        if depth > 8 || entries.len() >= 500 {
            return Ok(());
        }
        let mut children =
            fs::read_dir(root.join(relative))?.collect::<std::io::Result<Vec<_>>>()?;
        children.sort_by_key(|e| {
            (
                !e.file_type().is_ok_and(|kind| kind.is_dir()),
                e.file_name(),
            )
        });
        for child in children {
            if entries.len() >= 500 {
                break;
            }
            let name = child.file_name();
            let name = name.to_string_lossy();
            let kind = child.file_type()?;
            let path = relative.join(name.as_ref());
            if kind.is_symlink()
                || secret_file(&path)
                || [
                    "target",
                    ".git",
                    ".miblo",
                    "node_modules",
                    ".idea",
                    "__pycache__",
                    ".DS_Store",
                ]
                .contains(&name.as_ref())
            {
                continue;
            }
            let dir = kind.is_dir();
            if !dir && !kind.is_file() {
                continue;
            }
            let open = expanded.contains(&path);
            entries.push(Entry {
                path: path.clone(),
                label: format!(
                    "{}{} {}",
                    "  ".repeat(depth),
                    if dir {
                        if open { "▾" } else { "▸" }
                    } else {
                        " "
                    },
                    clean(&name)
                ),
                dir,
            });
            if dir && open {
                visit(root, &path, expanded, entries, depth + 1)?;
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    visit(root, Path::new(""), expanded, &mut entries, 0)?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn screen(app: &mut App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| view::draw(frame, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn workspace_renders_panels_dialogs_and_small_terminals() {
        let mut app = App::new();
        let rendered = screen(&mut app, 130, 40);
        for label in [
            "MIBLO",
            "Modèles / tables",
            "Fichiers",
            "Atelier API",
            "Logs",
            "État",
        ] {
            assert!(rendered.contains(label), "missing {label}");
        }
        app.palette();
        assert!(screen(&mut app, 80, 24).contains("Actions"));
        assert!(screen(&mut app, 40, 12).contains("Agrandis le terminal"));
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(app.dialog.is_none());
        app.command("theme").unwrap();
        assert_eq!(app.theme, 1);
        app.command("unknown-command").unwrap_err();
        assert!(!app.quit);
        app.input(
            forms::InputKind::DbPassword,
            "Mot de passe",
            "not-visible-secret",
        );
        let rendered = screen(&mut app, 130, 40);
        assert!(!rendered.contains("not-visible-secret"));
        assert!(rendered.contains("••••"));
    }

    #[test]
    fn explorer_ignores_private_env_and_refuses_external_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::new("sample".into(), 3000, Vec::new());
        fs::write(
            dir.path().join("miblo.yaml"),
            serde_yaml::to_string(&project).unwrap(),
        )
        .unwrap();
        fs::write(dir.path().join(".env"), "SECRET=hidden").unwrap();
        fs::write(dir.path().join(".env.example"), "EXAMPLE=value").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "// preview\nfn main() {}\n").unwrap();
        let mut app = App::new();
        app.open_project(dir.path()).unwrap();
        assert!(
            !app.entries
                .iter()
                .any(|entry| entry.path == Path::new(".env"))
        );
        assert!(
            app.entries
                .iter()
                .any(|entry| entry.path == Path::new(".env.example"))
        );
        assert_eq!(app.code[0], "// preview");
        assert!(app.load_file(Path::new(".env")).is_err());
        let outside = tempfile::NamedTempFile::new().unwrap();
        assert!(app.load_file(outside.path()).is_err());
    }
}
