mod forms;
mod http;
mod process;
mod request_form;
mod route_form;
mod schema;
mod splash;
mod status;
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
    Routes,
    Response,
    Tables,
    Schema,
}
#[derive(Clone)]
struct Entry {
    path: PathBuf,
    label: String,
    dir: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JobKind {
    Create,
    Add,
    Edit,
    Route,
    Templates,
    Database,
    Doctor,
}

impl JobKind {
    // Log prefixes and completion handling use stable identifiers, not labels.
    fn id(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Add => "add",
            Self::Edit => "edit",
            Self::Route => "route",
            Self::Templates => "templates",
            Self::Database => "database",
            Self::Doctor => "doctor",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Create => tr!("Project creation", "Création"),
            Self::Add => tr!("Add model", "Ajout de ressource"),
            Self::Edit => tr!("Update model", "Modification du modèle"),
            Self::Route => tr!("Route creation", "Création de route"),
            Self::Templates => "Templates",
            Self::Database => tr!("Database", "Base de données"),
            Self::Doctor => tr!("Diagnostics", "Diagnostic"),
        }
    }
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
    status: status::ProjectState,
    notice: String,
    theme: usize,
    dialog: Option<forms::Dialog>,
    new_draft: forms::NewDraft,
    add_draft: forms::AddDraft,
    route_draft: crate::config::CustomRoute,
    api: Option<Process>,
    job: Option<Process>,
    job_kind: Option<JobKind>,
    job_destination: Option<PathBuf>,
    sender: SyncSender<String>,
    receiver: Receiver<String>,
    quit: bool,
    editor_requested: bool,
    db_draft: database::MysqlSettings,
    db_status: status::DatabaseNotice,
    db_job: Option<Action>,
    migrate_after_add: bool,
    routes: Vec<http::Request>,
    route_selected: usize,
    request_draft: http::Request,
    tester: http::Tester,
    monitor: status::Monitor,
    base_url: String,
    api_ready: bool,
    api_failure: Option<String>,
    restart_required: bool,
    show_code: bool,
    code_visible: bool,
    show_schema: bool,
    schema: schema::Browser,
    template_draft: forms::TemplateDraft,
    resume_route_after_template_update: bool,
}

pub fn run(project: Option<PathBuf>) -> Result<()> {
    ensure!(
        wizard::interactive(),
        "The workspace requires a terminal; use miblo --help for scriptable commands"
    );
    let mut app = App::new();
    ratatui::run(|terminal| -> Result<()> {
        if !splash::show(terminal)? {
            return Ok(());
        }
        if let Some(path) = project {
            app.open_project(&path)?;
        } else if Path::new("miblo.yaml").is_file() {
            app.open_project(Path::new("."))?;
        }
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
    fn choose_language(&mut self) {
        let mut description = vec![
            tr!(
                "Applies immediately. Saved for your next launch.",
                "Appliqué immédiatement. Conservé pour le prochain lancement."
            )
            .into(),
        ];
        if std::env::var_os("MIBLO_LANGUAGE").is_some() {
            description.push(tr!(
                "MIBLO_LANGUAGE overrides the saved preference at startup. Unset it to use your saved choice.",
                "MIBLO_LANGUAGE prime au démarrage. Retire cette variable pour utiliser le choix enregistré."
            ).into());
        }
        self.choose(
            forms::ChoiceKind::Language,
            "Language / Langue",
            &["English", "Français"],
            description,
            usize::from(crate::i18n::current() == crate::i18n::Language::Fr),
        );
    }

    fn change_language(&mut self, language: crate::i18n::Language) -> Result<()> {
        crate::i18n::set(language);
        self.dialog = None;
        // Only the preference changes: the project, running API and background
        // jobs keep their state. Older logs remain as originally recorded.
        if let Err(error) = crate::i18n::Store::discover().and_then(|store| store.save(language)) {
            self.report(trf!(
                "Language changed for this session, but the preference could not be saved: {error:#}",
                "Langue changée pour cette session, mais la préférence n’a pas pu être enregistrée : {error:#}"
            ));
            return Ok(());
        }
        self.notice = tr!(
            "Language saved: English · L to change",
            "Langue enregistrée : français · L pour changer"
        )
        .into();
        Ok(())
    }

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
            status: status::ProjectState::Closed,
            notice: tr!(
                "n: create an API • o: open a project • Space: actions • ?: help",
                "n : créer une API • o : ouvrir un projet • Espace : actions • ? : aide"
            )
            .into(),
            theme: 0,
            dialog: None,
            new_draft: forms::NewDraft::default(),
            add_draft: forms::AddDraft::default(),
            route_draft: crate::config::CustomRoute::default(),
            api: None,
            job: None,
            job_kind: None,
            job_destination: None,
            sender,
            receiver,
            quit: false,
            editor_requested: false,
            db_draft: database::MysqlSettings::default(),
            db_status: status::DatabaseNotice::Unchecked,
            db_job: None,
            migrate_after_add: false,
            routes: vec![],
            route_selected: 0,
            request_draft: http::Request::default(),
            tester: http::Tester::default(),
            monitor: status::Monitor::default(),
            base_url: String::new(),
            api_ready: false,
            api_failure: None,
            restart_required: false,
            show_code: true,
            code_visible: true,
            show_schema: false,
            schema: schema::Browser::default(),
            template_draft: forms::TemplateDraft::default(),
            resume_route_after_template_update: false,
        }
    }

    fn open_project(&mut self, path: &Path) -> Result<()> {
        ensure!(
            self.api.is_none() && self.job.is_none() && !self.tester.pending,
            trf!(
                "Stop the API and wait for the current action before changing projects",
                "Arrête l’API et attends la fin de l’action avant de changer de projet"
            )
        );
        let root = path
            .canonicalize()
            .context(tr!("Directory not found", "Dossier introuvable"))?;
        let project = Project::load(&root.join("miblo.yaml"))?;
        self.root = Some(root);
        self.project = Some(project);
        self.schema = schema::Browser::default();
        self.show_schema = false;
        self.tester = http::Tester::default();
        self.route_selected = 0;
        self.api_failure = None;
        self.api_ready = false;
        self.restart_required = false;
        self.resume_route_after_template_update = false;
        self.db_status = status::DatabaseNotice::Unchecked;
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
        self.notice = tr!(
            "Project opened • Tab: switch panel • Space: actions",
            "Projet ouvert • Tab : changer de panneau • Espace : actions"
        )
        .into();
        Ok(())
    }

    fn refresh(&mut self) -> Result<()> {
        let Some(root) = self.root.clone() else {
            return Ok(());
        };
        self.project = Some(Project::load(&root.join("miblo.yaml"))?);
        self.schema.project(self.project.as_ref().unwrap());
        if self.show_schema {
            self.schema.refresh(&root);
        }
        self.routes = http::routes(self.project.as_ref().unwrap());
        self.route_selected = self.route_selected.min(self.routes.len().saturating_sub(1));
        self.base_url = crate::runtime::http_base(&root).unwrap_or_default();
        self.monitor.invalidate();
        self.status = match generation::inspect(&root) {
            Ok(_) => status::ProjectState::Ready,
            Err(e) => status::ProjectState::Issue(e.to_string()),
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
        let root = self
            .root
            .as_ref()
            .context(tr!("Open a project first", "Ouvre un projet d’abord"))?;
        let path = root.join(relative).canonicalize()?;
        ensure!(
            path.starts_with(root) && path.is_file(),
            trf!(
                "The file must belong to this project",
                "Le fichier doit appartenir au projet"
            )
        );
        ensure!(
            !secret_file(relative),
            trf!(
                "Private .env files are hidden",
                "Les fichiers .env privés ne sont pas affichés"
            )
        );
        let mut bytes = Vec::new();
        fs::File::open(&path)?
            .take(128 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        let truncated = bytes.len() > 128 * 1024;
        bytes.truncate(128 * 1024);
        ensure!(
            !bytes.contains(&0),
            trf!("This is a binary file", "Ce fichier est binaire")
        );
        self.code = String::from_utf8_lossy(&bytes).lines().map(clean).collect();
        if truncated {
            self.code.push(
                tr!(
                    "… preview limited to 128 KiB — open the file in your editor to see more",
                    "… aperçu limité à 128 Kio — ouvre le fichier dans ton éditeur pour la suite"
                )
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
                .context(tr!("No model", "Aucune ressource"))?;
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
                self.resume_route_after_template_update = false;
                self.db_draft.password.clear();
                self.notice = tr!("Action cancelled", "Action annulée").into();
            } else {
                self.request_quit()?;
            }
            return Ok(());
        }
        if self.dialog.is_some() {
            return self.dialog_key(key);
        }
        match key.code {
            KeyCode::Char(':') => {
                self.input(forms::InputKind::Command, tr!("Command", "Commande"), "")
            }
            KeyCode::Char(' ') => self.palette(),
            KeyCode::Char('L') => self.choose_language(),
            KeyCode::Char('?') | KeyCode::F(1) => self.dialog = Some(forms::Dialog::Help),
            KeyCode::Tab => {
                self.cycle_focus(false);
            }
            KeyCode::BackTab => {
                self.cycle_focus(true);
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
            KeyCode::Enter if self.focus == Focus::Routes => self.start_route_test()?,
            KeyCode::Enter if self.focus == Focus::Tables => self.focus = Focus::Schema,
            KeyCode::Char('d') => self.toggle_schema()?,
            KeyCode::Char('p') if self.show_schema => self.schema.toggle_source(),
            KeyCode::Char('u') => self.start_template_update()?,
            KeyCode::Char('x') => self.start_route_test()?,
            KeyCode::Char('f') => self.custom_request()?,
            KeyCode::Char('v') => {
                self.show_code = !self.show_code;
                if !self.show_code && self.focus == Focus::Code {
                    self.focus = Focus::Routes;
                }
            }
            KeyCode::Char('n') => self.start_new()?,
            KeyCode::Char('g') => self.start_route_create()?,
            KeyCode::Char('a') => self.start_add()?,
            KeyCode::Char('m') => self.start_model_edit()?,
            KeyCode::Char('b') => self.database_menu()?,
            KeyCode::Char('o') => self.input(
                forms::InputKind::Open,
                tr!("Project directory", "Dossier du projet"),
                ".",
            ),
            KeyCode::Char('r') => self.run_api()?,
            KeyCode::Char('s') => self.stop_api(),
            KeyCode::Char('c') => self.cargo_job("check")?,
            KeyCode::Char('e') => self.request_editor()?,
            KeyCode::Char('t') => {
                self.theme = (self.theme + 1) % 3;
                self.notice = tr!(
                    "Theme changed for this session",
                    "Thème modifié pour cette session"
                )
                .into();
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
            Focus::Routes => (&mut self.route_selected, self.routes.len()),
            Focus::Response => (&mut self.tester.scroll, self.tester.lines.len()),
            Focus::Tables => {
                self.schema.scroll = 0;
                let length = self.schema.tables().len();
                (&mut self.schema.selected, length)
            }
            Focus::Schema => {
                let length = self.schema.details().len();
                (&mut self.schema.scroll, length)
            }
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
        self.schema.tick();
        if self.tester.tick() {
            self.received_response();
        }
        while let Ok(message) = self.receiver.try_recv() {
            if message.starts_with("[API]")
                && message.contains("Server running on")
                && self.api.is_some()
            {
                self.api_ready = true;
                self.monitor.invalidate();
            }
            self.log(message);
        }
        let finished = self
            .api
            .as_mut()
            .and_then(|process| process.poll().ok().flatten());
        if let Some(status) = finished {
            self.api = None;
            self.api_ready = false;
            self.api_failure = Some(status.to_string());
            self.monitor.invalidate();
            self.log(format!("API : {status}"));
        }
        let finished = self
            .job
            .as_mut()
            .and_then(|process| process.poll().ok().flatten());
        if let Some(status) = finished {
            let raw_label = self.job.take().map(|p| p.label.clone()).unwrap_or_default();
            let kind = self.job_kind.take();
            let label = match kind {
                Some(kind) => kind.label(),
                None => raw_label.as_str(),
            };
            let db_action = self.db_job.take();
            let migrate_after_add = std::mem::take(&mut self.migrate_after_add);
            self.notice = format!("{label} : {status}");
            self.log(self.notice.clone());
            if status.success() {
                if kind == Some(JobKind::Route) {
                    self.restart_required = self.api.is_some();
                }
                if migrate_after_add {
                    self.db_status = status::DatabaseNotice::Pending;
                    self.restart_required = self.api.is_some();
                }
                if let Some(action) = db_action {
                    self.db_status = if action == Action::Check {
                        status::DatabaseNotice::Connected
                    } else {
                        status::DatabaseNotice::Ready
                    };
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
                self.db_status = status::DatabaseNotice::Failed;
            } else if kind == Some(JobKind::Route) {
                self.route_creation_failed();
            }
            if kind == Some(JobKind::Templates) {
                self.template_update_finished(status.success());
            }
            if status.success() && kind == Some(JobKind::Route) {
                self.show_schema = false;
                self.focus = Focus::Routes;
                self.route_selected = self
                    .routes
                    .iter()
                    .position(|r| {
                        r.path == self.route_draft.path && r.method == self.route_draft.method
                    })
                    .unwrap_or(0);
                if let Err(error) = self.preview_route() {
                    self.report(format!("{error:#}"));
                } else {
                    self.notice = if self.route_draft.action.is_some() {
                        tr!("Route created • model CRUD and SQL connected • restart the API, then x to test", "Route créée • CRUD et SQL du modèle reliés • redémarre l’API puis x pour tester")
                    } else {
                        tr!("Route created • complete business logic with e • initial response HTTP 501", "Route créée • métier à compléter avec e • réponse initiale HTTP 501")
                    }.into();
                }
            }
            self.monitor.invalidate();
        }
        self.monitor
            .tick(self.root.as_deref(), &self.base_url, self.job.is_some());
    }

    fn ready_for_job(&self) -> Result<&Path> {
        ensure!(
            self.job.is_none(),
            trf!(
                "An action is already running",
                "Une action est déjà en cours"
            )
        );
        ensure!(
            !self.tester.pending,
            trf!(
                "Wait for the HTTP request to finish",
                "Attends la fin de la requête HTTP"
            )
        );
        self.root.as_deref().context(tr!(
            "No project open: n to create an API, o to open a project",
            "Aucun projet ouvert : n pour créer une API, o pour ouvrir un projet"
        ))
    }
    fn run_api(&mut self) -> Result<()> {
        ensure!(
            self.api.is_none(),
            trf!(
                "The API is already running in Miblo",
                "L’API est déjà lancée par Miblo"
            )
        );
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
        self.api_ready = false;
        self.api_failure = None;
        self.restart_required = false;
        self.monitor.invalidate();
        self.log(tr!("Starting the API…", "Lancement de l’API…").into());
        Ok(())
    }
    fn stop_api(&mut self) {
        self.api = None;
        self.api_ready = false;
        self.api_failure = None;
        self.restart_required = false;
        self.monitor.invalidate();
        self.log(tr!("API stopped", "API arrêtée").into());
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
        kind: JobKind,
        destination: Option<PathBuf>,
        remove_local_env: bool,
    ) -> Result<()> {
        ensure!(
            self.job.is_none(),
            trf!(
                "An action is already running",
                "Une action est déjà en cours"
            )
        );
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args(args)
            .env("MIBLO_LANGUAGE", crate::i18n::current().code());
        if remove_local_env {
            command.env_remove("MIBLO_TEMPLATE_DIR");
        }
        self.job = Some(Process::spawn(command, kind.id(), self.sender.clone())?);
        self.job_kind = Some(kind);
        self.monitor.invalidate();
        self.job_destination = destination;
        self.dialog = None;
        self.notice = trf!("{} in progress…", "{} en cours…", kind.label());
        Ok(())
    }

    fn request_editor(&mut self) -> Result<()> {
        ensure!(
            self.job.is_none(),
            trf!(
                "Wait for generation to finish before opening the editor",
                "Attends la fin de la génération avant d’ouvrir l’éditeur"
            )
        );
        if self.focus == Focus::Routes {
            self.preview_route()?;
        } else if self.focus == Focus::Response
            && let Some(route) = self.unimplemented_route()
        {
            let path = format!("src/resources/custom_routes/{}/service.rs", route.name);
            self.load_file(Path::new(&path))?;
        }
        ensure!(
            self.file.is_some(),
            trf!("Select a file first", "Sélectionne un fichier d’abord")
        );
        self.editor_requested = true;
        Ok(())
    }
    fn open_editor(&self) -> Result<()> {
        let root = self
            .root
            .as_ref()
            .context(tr!("No project", "Aucun projet"))?;
        let path = root
            .join(
                self.file
                    .as_ref()
                    .context(tr!("No file", "Aucun fichier"))?,
            )
            .canonicalize()?;
        ensure!(
            path.starts_with(root),
            trf!(
                "The file must belong to this project",
                "Le fichier doit appartenir au projet"
            )
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
        let words = shell_words::split(&editor).context(tr!(
            "Invalid VISUAL/EDITOR setting",
            "VISUAL/EDITOR est invalide"
        ))?;
        let executable = words
            .first()
            .context(tr!("VISUAL/EDITOR is empty", "VISUAL/EDITOR est vide"))?;
        let status = Command::new(executable)
            .args(&words[1..])
            .arg(path)
            .current_dir(root)
            .status()?;
        ensure!(
            status.success(),
            trf!(
                "The editor exited with {status}",
                "L’éditeur s’est terminé avec {status}"
            )
        );
        Ok(())
    }

    fn request_quit(&mut self) -> Result<()> {
        ensure!(
            !self.tester.pending,
            trf!(
                "Wait for the HTTP request to finish before quitting",
                "Attends la fin de la requête HTTP avant de quitter"
            )
        );
        ensure!(
            self.job.is_none(),
            trf!(
                "An action is running; wait for it to finish so file writes are not interrupted",
                "Une action est en cours ; attends sa fin pour quitter sans interrompre l’écriture des fichiers"
            )
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
            "model" => self.start_model_edit()?,
            "route-new" => self.start_route_create()?,
            "routes" => {
                self.show_schema = false;
                self.focus = Focus::Routes;
            }
            "schema" => self.toggle_schema()?,
            "templates" => self.start_template_update()?,
            "route" => self.start_route_test()?,
            "request" | "http" => self.custom_request()?,
            "code" => self.show_code = !self.show_code,
            "db" | "database" => self.database_menu()?,
            "migrate" => self.confirm_database_action(Action::Migrate)?,
            "open" if argument.is_empty() => self.input(
                forms::InputKind::Open,
                tr!("Project directory", "Dossier du projet"),
                ".",
            ),
            "open" => self.open_project(Path::new(argument.trim()))?,
            "run" | "start" => self.run_api()?,
            "stop" => self.stop_api(),
            "check" => self.cargo_job("check")?,
            "test" => self.cargo_job("test")?,
            "doctor" => {
                let root = self.ready_for_job()?.as_os_str().to_owned();
                self.cli_job(
                    vec!["doctor".into(), "--project".into(), root],
                    JobKind::Doctor,
                    None,
                    false,
                )?;
            }
            "edit" | "e" => self.request_editor()?,
            "refresh" => self.refresh()?,
            "theme" => self.theme = (self.theme + 1) % 3,
            "language" | "langue" => self.choose_language(),
            "help" => self.dialog = Some(forms::Dialog::Help),
            _ => anyhow::bail!(trf!(
                "Unknown command. :help shows the available commands.",
                "Commande inconnue. :help affiche les commandes disponibles."
            )),
        }
        Ok(())
    }

    fn cycle_focus(&mut self, backwards: bool) {
        let mut order = vec![Focus::Resources, Focus::Files];
        if self.show_schema {
            order.extend([Focus::Tables, Focus::Schema]);
        } else {
            order.extend([Focus::Routes, Focus::Response]);
        }
        if self.code_visible {
            order.push(Focus::Code);
        }
        order.push(Focus::Logs);
        let index = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        self.focus = order[(index + if backwards { order.len() - 1 } else { 1 }) % order.len()];
    }

    fn toggle_schema(&mut self) -> Result<()> {
        let root = self.root.as_ref().context(tr!(
            "Open a project to view its database",
            "Ouvre un projet pour voir sa base de données"
        ))?;
        self.show_schema = !self.show_schema;
        if self.show_schema {
            self.schema.refresh(root);
            self.focus = Focus::Tables;
            self.notice = tr!(
                "Live database schema · p: declared models · F5: refresh · d: routes",
                "Schéma réel de la base · p : modèles déclarés · F5 : actualiser · d : routes"
            )
            .into();
        } else {
            self.focus = Focus::Routes;
        }
        Ok(())
    }

    fn api_badge(&self) -> status::Badge {
        use status::{Badge, HttpHealth, Tone};
        if self.root.is_none() {
            return Badge::new(
                Tone::Neutral,
                tr!("No project", "Aucun projet"),
                tr!("n create · o open", "n créer · o ouvrir"),
            );
        }
        if let Some(error) = &self.api_failure {
            return Badge::new(
                Tone::Error,
                tr!("API exited", "API interrompue"),
                trf!("{error} · check logs", "{error} · consulter les logs"),
            );
        }
        if self.base_url.is_empty() {
            return Badge::new(
                Tone::Error,
                tr!("Invalid HTTP address", "Adresse HTTP invalide"),
                tr!(
                    "Check SERVER_ADDRESS / SERVER_PORT, then F5",
                    "Vérifie SERVER_ADDRESS / SERVER_PORT puis F5"
                ),
            );
        }
        if self.api.is_some()
            && !self.api_ready
            && !matches!(self.monitor.api, HttpHealth::Responding(..))
        {
            return Badge::new(
                Tone::Warning,
                tr!("Starting", "Démarrage en cours"),
                tr!(
                    "Compiling / connecting to the database…",
                    "Compilation / connexion à la base…"
                ),
            );
        }
        match &self.monitor.api {
            HttpHealth::Responding(code, elapsed) => {
                if self.restart_required {
                    Badge::new(
                        Tone::Warning,
                        tr!("API restart required", "API à redémarrer"),
                        tr!(
                            "API responds · s then r to load new routes",
                            "API répond · s puis r pour charger les nouvelles routes"
                        ),
                    )
                } else {
                    Badge::new(
                        Tone::Good,
                        if self.api.is_some() {
                            tr!("API running", "API en cours")
                        } else {
                            tr!("API available (external)", "API accessible (externe)")
                        },
                        format!("GET /health → HTTP {code} · {elapsed} ms"),
                    )
                }
            }
            HttpHealth::Failed(code) if self.api.is_some() || code.is_some() => Badge::new(
                Tone::Error,
                tr!("API not responding", "API ne répond pas"),
                code.map_or(
                    tr!(
                        "No HTTP response · check logs",
                        "Aucune réponse HTTP · consulter les logs"
                    )
                    .into(),
                    |s| format!("GET /health → HTTP {s}"),
                ),
            ),
            HttpHealth::Unknown if self.api.is_some() => Badge::new(
                Tone::Warning,
                tr!("Checking HTTP…", "Vérification HTTP…"),
                tr!(
                    "Process started; waiting for an HTTP response",
                    "Le processus a démarré, réponse HTTP attendue"
                ),
            ),
            HttpHealth::Unknown => Badge::new(
                Tone::Neutral,
                tr!("Checking HTTP…", "Vérification HTTP…"),
                tr!("Checking /health", "Contrôle de /health en cours"),
            ),
            _ => Badge::new(
                Tone::Neutral,
                tr!("API stopped", "API arrêtée"),
                tr!(
                    "r start API · x test a route",
                    "r lancer l’API · x tester une route"
                ),
            ),
        }
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
            tr!("Models · m edit", "Modèles · m modifier"),
            tr!("Files", "Fichiers"),
            tr!("API workspace", "Atelier API"),
            "Logs",
            tr!("status", "État"),
        ] {
            assert!(rendered.contains(label), "missing {label}");
        }
        app.palette();
        assert!(screen(&mut app, 80, 24).contains(tr!("Actions", "Actions")));
        assert!(
            screen(&mut app, 40, 12).contains(tr!("Resize the terminal", "Agrandis le terminal"))
        );
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(app.dialog.is_none());
        app.command("theme").unwrap();
        assert_eq!(app.theme, 1);
        app.command("unknown-command").unwrap_err();
        assert!(!app.quit);
        app.input(
            forms::InputKind::DbPassword,
            tr!("Password", "Mot de passe"),
            "not-visible-secret",
        );
        let rendered = screen(&mut app, 130, 40);
        assert!(!rendered.contains("not-visible-secret"));
        assert!(rendered.contains("••••"));
    }

    #[test]
    fn dashboard_keeps_status_visible_and_code_small() {
        let mut app = App::new();
        app.root = Some(PathBuf::from("/tmp/example"));
        app.base_url = "http://127.0.0.1:3000".into();
        app.project = Some(Project::new("example".into(), 3000, vec![]));
        app.routes = http::routes(app.project.as_ref().unwrap());
        app.file = Some("src/main.rs".into());
        app.code = (0..100).map(|i| format!("code-line-{i:03}")).collect();
        app.monitor.api = status::HttpHealth::Responding(200, 4);
        app.monitor.database = database::health::Health::Pending(2);
        assert_eq!(app.api_badge().tone, status::Tone::Good);
        let rendered = screen(&mut app, 130, 40);
        for label in [
            tr!("API available (external)", "API accessible (externe)"),
            tr!("Database update required", "Mise à jour BDD requise"),
            "Routes",
            tr!("HTTP response", "Réponse HTTP"),
            "code-line-003",
        ] {
            assert!(rendered.contains(label), "missing {label}");
        }
        assert!(!rendered.contains("code-line-004"));
        app.command("code").unwrap();
        assert!(!screen(&mut app, 130, 40).contains("code-line-000"));
        let compact = screen(&mut app, 80, 24);
        for label in [
            tr!("API status", "État API"),
            tr!("DB status", "État BDD"),
            "Routes",
            tr!("HTTP response", "Réponse HTTP"),
        ] {
            assert!(compact.contains(label));
        }
        app.monitor.api = status::HttpHealth::Failed(Some(503));
        assert_eq!(app.api_badge().tone, status::Tone::Error);
        app.monitor.api = status::HttpHealth::Failed(None);
        assert_eq!(app.api_badge().tone, status::Tone::Neutral);
        app.api_failure = Some("exit status: 1".into());
        assert_eq!(app.api_badge().tone, status::Tone::Error);
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

    #[test]
    fn schema_view_shows_keys_and_relations_on_compact_terminals() {
        use crate::config::{Resource, parse_field};
        let mut app = App::new();
        let project = Project::new(
            "schema-test".into(),
            3000,
            vec![
                Resource::new("Group".into(), vec![parse_field("name:string").unwrap()]),
                Resource::new(
                    "User".into(),
                    vec![parse_field("group_id:int:ref=Group").unwrap()],
                ),
            ],
        );
        app.schema.project(&project);
        app.project = Some(project);
        app.show_schema = true;
        app.schema.planned = true;
        app.schema.selected = 1;
        app.focus = Focus::Schema;
        let rendered = screen(&mut app, 80, 24);
        for label in [
            tr!("Declared schema", "Schéma déclaré"),
            "TABLE users",
            "PK",
        ] {
            assert!(rendered.contains(label), "missing {label}");
        }
        app.schema.scroll = 3;
        assert!(screen(&mut app, 80, 24).contains("group_id"));
        app.schema.scroll = 6;
        assert!(screen(&mut app, 80, 24).contains("group_id → groups.id"));
        assert!(!app.code_visible);
    }
}
