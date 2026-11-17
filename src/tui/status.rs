use crate::database::health::{self, Health};
use reqwest::{blocking::Client, redirect::Policy};
use std::{
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tone {
    Good,
    Warning,
    Error,
    Neutral,
}

pub struct Badge {
    pub tone: Tone,
    pub title: String,
    pub detail: String,
}
impl Badge {
    pub fn new(tone: Tone, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            tone,
            title: title.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum HttpHealth {
    Unknown,
    Responding(u16, u128),
    Failed(Option<u16>),
}
fn probe(base: &str) -> HttpHealth {
    let start = Instant::now();
    let Ok(client) = Client::builder()
        .timeout(Duration::from_millis(800))
        .redirect(Policy::none())
        .no_proxy()
        .build()
    else {
        return HttpHealth::Failed(None);
    };
    match client.get(format!("{base}/health")).send() {
        Ok(response) if response.status().is_success() => {
            HttpHealth::Responding(response.status().as_u16(), start.elapsed().as_millis())
        }
        Ok(response) => HttpHealth::Failed(Some(response.status().as_u16())),
        Err(_) => HttpHealth::Failed(None),
    }
}

pub struct Monitor {
    pub api: HttpHealth,
    pub database: Health,
    epoch: u64,
    pending: bool,
    checked: Option<Instant>,
    sender: mpsc::SyncSender<(u64, HttpHealth, Health)>,
    receiver: mpsc::Receiver<(u64, HttpHealth, Health)>,
}
impl Default for Monitor {
    fn default() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        Self {
            api: HttpHealth::Unknown,
            database: Health::Unknown,
            epoch: 0,
            pending: false,
            checked: None,
            sender,
            receiver,
        }
    }
}
impl Monitor {
    pub fn invalidate(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.checked = None;
        self.database = Health::Unknown;
        self.api = HttpHealth::Unknown;
    }
    pub fn tick(&mut self, root: Option<&Path>, base: &str, busy: bool) {
        if let Ok((epoch, api, database)) = self.receiver.try_recv() {
            self.pending = false;
            if epoch == self.epoch && !busy {
                self.api = api;
                self.database = database;
                self.checked = Some(Instant::now());
            }
        }
        if busy
            || self.pending
            || self
                .checked
                .is_some_and(|at| at.elapsed() < Duration::from_secs(3))
        {
            return;
        }
        let Some(root) = root else {
            return;
        };
        self.pending = true;
        let root = root.to_owned();
        let base = base.to_owned();
        let epoch = self.epoch;
        let sender = self.sender.clone();
        thread::spawn(move || {
            let api = if base.is_empty() {
                HttpHealth::Unknown
            } else {
                probe(&base)
            };
            let database = health::inspect(&root);
            let _ = sender.send((epoch, api, database));
        });
    }
}

pub fn database_badge(health: &Health, busy: bool) -> Badge {
    if busy {
        return Badge::new(
            Tone::Warning,
            "Action BDD en cours",
            "Vérification après la fin de l’action",
        );
    }
    match health {
        Health::Unknown => Badge::new(
            Tone::Neutral,
            "Vérification BDD…",
            "Contrôle des migrations réellement appliquées",
        ),
        Health::Ready => Badge::new(
            Tone::Good,
            "Migrations à jour",
            "Base connectée · aucune mise à jour en attente",
        ),
        Health::Pending(count) => Badge::new(
            Tone::Warning,
            "Mise à jour BDD requise",
            format!("{count} migration(s) en attente · b pour appliquer"),
        ),
        Health::Initialize => Badge::new(
            Tone::Warning,
            "Base à initialiser",
            "b → Initialiser la base et ses tables",
        ),
        Health::Unavailable(error) => Badge::new(
            Tone::Error,
            "Base inaccessible",
            format!("{error} · b configurer"),
        ),
        Health::Drift(error) => Badge::new(Tone::Error, "Migration incohérente", error),
    }
}
