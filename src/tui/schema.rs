use crate::{
    config::Project,
    database::schema::{self, Schema, Table},
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

pub struct Browser {
    pub planned: bool,
    pub selected: usize,
    pub scroll: usize,
    pub actual: Option<Schema>,
    pub error: Option<String>,
    declared: Vec<Table>,
    pending: bool,
    epoch: u64,
    queued: Option<PathBuf>,
    sender: mpsc::SyncSender<(u64, Result<Schema, String>)>,
    receiver: mpsc::Receiver<(u64, Result<Schema, String>)>,
}
impl Default for Browser {
    fn default() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        Self {
            planned: false,
            selected: 0,
            scroll: 0,
            actual: None,
            error: None,
            declared: vec![],
            pending: false,
            epoch: 0,
            queued: None,
            sender,
            receiver,
        }
    }
}
impl Browser {
    pub fn project(&mut self, project: &Project) {
        self.declared = schema::declared(project);
    }
    pub fn tables(&self) -> &[Table] {
        if self.planned {
            &self.declared
        } else {
            self.actual.as_ref().map_or(&[], |s| &s.tables)
        }
    }
    pub fn loading(&self) -> bool {
        self.pending || self.queued.is_some()
    }
    pub fn refresh(&mut self, root: &Path) {
        self.epoch = self.epoch.wrapping_add(1);
        self.actual = None;
        self.error = None;
        self.queued = Some(root.to_owned());
    }
    pub fn tick(&mut self) {
        if let Ok((epoch, result)) = self.receiver.try_recv() {
            self.pending = false;
            if epoch == self.epoch {
                match result {
                    Ok(schema) => self.actual = Some(schema),
                    Err(error) => self.error = Some(error),
                }
                self.selected = self.selected.min(self.tables().len().saturating_sub(1));
                self.scroll = 0;
            }
        }
        if !self.pending
            && let Some(root) = self.queued.take()
        {
            self.pending = true;
            let sender = self.sender.clone();
            let epoch = self.epoch;
            thread::spawn(move || {
                let _ = sender.send((
                    epoch,
                    schema::inspect(&root).map_err(|error| error.to_string()),
                ));
            });
        }
    }
    pub fn toggle_source(&mut self) {
        self.planned = !self.planned;
        self.selected = 0;
        self.scroll = 0;
    }
    pub fn details(&self) -> Vec<String> {
        let Some(table) = self.tables().get(self.selected) else {
            return vec![
                "Aucune table à afficher.".into(),
                "b : initialiser / migrer · p : schéma déclaré".into(),
            ];
        };
        let mut lines = vec![
            format!("TABLE {}", table.name),
            "PK : clé primaire · FK : clé étrangère".into(),
            String::new(),
        ];
        for column in &table.columns {
            let fk = table
                .foreign_keys
                .iter()
                .any(|key| key.columns.contains(&column.name));
            let label = match (column.primary_key > 0, fk) {
                (true, true) => "PK/FK",
                (true, false) => "PK",
                (false, true) => "FK",
                _ => "",
            };
            lines.push(format!(
                "{label:<5} {:<18} {}{}",
                column.name,
                column.sql_type,
                if column.nullable {
                    " · NULL autorisé"
                } else {
                    " · NOT NULL"
                }
            ));
        }
        lines.extend([String::new(), "RELATIONS SORTANTES".into()]);
        if table.foreign_keys.is_empty() {
            lines.push("Aucune clé étrangère".into());
        }
        for key in &table.foreign_keys {
            lines.push(format!(
                "{} → {}.{}",
                key.columns.join(", "),
                key.target,
                key.target_columns.join(", ")
            ));
            lines.push(format!(
                "  Suppression : {} · modification : {}",
                key.on_delete, key.on_update
            ));
        }
        lines.extend([String::new(), "RÉFÉRENCÉE PAR".into()]);
        let start = lines.len();
        for source in self.tables() {
            for key in &source.foreign_keys {
                if key.target == table.name {
                    lines.push(format!(
                        "{}.{} → {}.{}",
                        source.name,
                        key.columns.join(", "),
                        table.name,
                        key.target_columns.join(", ")
                    ));
                }
            }
        }
        if start == lines.len() {
            lines.push("Aucune autre table".into());
        }
        lines
    }
}
