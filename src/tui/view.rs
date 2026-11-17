use super::*;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    panel: Color,
    foreground: Color,
    muted: Color,
    accent: Color,
    border: Color,
    selected: Color,
}
fn palette(index: usize) -> Palette {
    match index {
        1 => Palette {
            background: Color::Rgb(12, 26, 24),
            panel: Color::Rgb(18, 36, 31),
            foreground: Color::Rgb(224, 239, 219),
            muted: Color::Rgb(130, 160, 143),
            accent: Color::Rgb(161, 224, 118),
            border: Color::Rgb(46, 77, 63),
            selected: Color::Rgb(38, 67, 48),
        },
        2 => Palette {
            background: Color::Rgb(241, 243, 248),
            panel: Color::Rgb(252, 252, 255),
            foreground: Color::Rgb(31, 40, 60),
            muted: Color::Rgb(85, 100, 125),
            accent: Color::Rgb(35, 83, 177),
            border: Color::Rgb(170, 183, 205),
            selected: Color::Rgb(213, 228, 249),
        },
        _ => Palette {
            background: Color::Rgb(15, 18, 29),
            panel: Color::Rgb(21, 26, 40),
            foreground: Color::Rgb(224, 230, 245),
            muted: Color::Rgb(131, 145, 170),
            accent: Color::Rgb(109, 218, 205),
            border: Color::Rgb(47, 61, 85),
            selected: Color::Rgb(37, 58, 77),
        },
    }
}
fn panel<'a>(title: impl Into<Line<'a>>, focused: bool, colors: Palette) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused {
            colors.accent
        } else {
            colors.border
        }))
        .style(Style::default().fg(colors.foreground).bg(colors.panel))
}

pub(super) fn draw(frame: &mut Frame, app: &mut App) {
    let colors = palette(app.theme);
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(colors.background).fg(colors.foreground)),
        area,
    );
    if area.width < 70 || area.height < 20 {
        frame.render_widget(
            Paragraph::new("MIBLO\nAgrandis le terminal à au moins 70 × 20.\nq pour quitter")
                .style(Style::default().fg(colors.accent))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(if area.height > 30 { 8 } else { 5 }),
        Constraint::Length(3),
    ])
    .split(area);
    let project = app
        .project
        .as_ref()
        .map_or("Bienvenue", |p| p.name.as_str());
    let title = Line::from(vec![
        Span::styled(
            " MIBLO ",
            Style::default()
                .fg(colors.background)
                .bg(colors.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("   {project}   ")),
        Span::styled(
            format!(
                "Rust / Axum / {}",
                app.project
                    .as_ref()
                    .map_or("SQLite · MySQL", |p| p.database().name())
            ),
            Style::default().fg(colors.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(title).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(colors.border)),
        ),
        layout[0],
    );
    let wide = area.width >= 110;
    let columns = if wide {
        Layout::horizontal([
            Constraint::Length(27),
            Constraint::Min(30),
            Constraint::Length(28),
        ])
        .split(layout[1])
    } else {
        Layout::horizontal([Constraint::Length(24), Constraint::Min(30)]).split(layout[1])
    };
    let sidebar = Layout::vertical([Constraint::Length(7), Constraint::Min(3)]).split(columns[0]);
    let resources: Vec<ListItem> = app
        .project
        .as_ref()
        .map(|project| {
            project
                .resources
                .iter()
                .map(|resource| {
                    ListItem::new(vec![
                        Line::from(resource.name.clone()),
                        Line::from(Span::styled(
                            resource.path.clone(),
                            Style::default().fg(colors.muted),
                        )),
                    ])
                })
                .collect()
        })
        .unwrap_or_default();
    let mut resource_state = ListState::default().with_selected(if resources.is_empty() {
        None
    } else {
        Some(app.resource_selected)
    });
    frame.render_stateful_widget(
        List::new(resources)
            .block(panel(
                " Modèles / tables ",
                app.focus == Focus::Resources,
                colors,
            ))
            .highlight_style(Style::default().bg(colors.selected).fg(colors.accent))
            .highlight_symbol("› "),
        sidebar[0],
        &mut resource_state,
    );
    let files: Vec<_> = app
        .entries
        .iter()
        .map(|entry| {
            ListItem::new(entry.label.clone()).style(Style::default().fg(if entry.dir {
                colors.accent
            } else {
                colors.foreground
            }))
        })
        .collect();
    let mut file_state = ListState::default().with_selected(if app.entries.is_empty() {
        None
    } else {
        Some(app.selected)
    });
    frame.render_stateful_widget(
        List::new(files)
            .block(panel(" Fichiers ", app.focus == Focus::Files, colors))
            .highlight_style(Style::default().bg(colors.selected))
            .highlight_symbol("›"),
        sidebar[1],
        &mut file_state,
    );
    if app.file.is_some() {
        let filename = app
            .file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let height = columns[1].height.saturating_sub(2) as usize;
        let lines: Vec<_> = app
            .code
            .iter()
            .enumerate()
            .skip(app.code_scroll)
            .take(height)
            .map(|(index, code)| {
                let color =
                    if code.trim_start().starts_with("//") || code.trim_start().starts_with('#') {
                        colors.muted
                    } else {
                        colors.foreground
                    };
                let visible = code
                    .chars()
                    .skip(app.code_horizontal as usize)
                    .collect::<String>();
                Line::from(vec![
                    Span::styled(
                        format!("{:>4} │ ", index + 1),
                        Style::default().fg(colors.muted),
                    ),
                    Span::styled(visible, Style::default().fg(color)),
                ])
            })
            .collect();
        frame.render_widget(
            Paragraph::new(lines).block(panel(
                format!(" {filename} · lecture seule "),
                app.focus == Focus::Code,
                colors,
            )),
            columns[1],
        );
    } else {
        let welcome = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Ton API, au même endroit.",
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  n    Créer une API"),
            Line::from("  o    Ouvrir un projet"),
            Line::from("  a    Ajouter un modèle / une table"),
            Line::from("  b    Configurer la base / les migrations"),
            Line::from(""),
            Line::from("  Espace    Palette d’actions"),
            Line::from("  :         Barre de commandes"),
            Line::from("  ?         Raccourcis et aide"),
            Line::from(""),
            Line::from(Span::styled(
                "  Flèches ou j/k • Tab entre les panneaux",
                Style::default().fg(colors.muted),
            )),
        ];
        frame.render_widget(
            Paragraph::new(welcome).block(panel(" Atelier API ", app.focus == Focus::Code, colors)),
            columns[1],
        );
    }
    if wide {
        let resource_count = app.project.as_ref().map_or(0, |p| p.resources.len());
        let status = vec![
            Line::from(Span::styled(" API", Style::default().fg(colors.accent))),
            Line::from(format!(" {}", app.api_status)),
            Line::from(""),
            Line::from(Span::styled(" PROJET", Style::default().fg(colors.accent))),
            Line::from(format!(" {}", app.status)),
            Line::from(format!(" {resource_count} ressource(s)")),
            Line::from(""),
            Line::from(Span::styled(" ACTION", Style::default().fg(colors.accent))),
            Line::from(
                app.job
                    .as_ref()
                    .map_or(" Aucune action en cours".into(), |p| {
                        format!(" {}…", p.label)
                    }),
            ),
            Line::from(""),
            Line::from(" r lancer  •  s arrêter"),
            Line::from(" c vérifier • e éditeur"),
            Line::from(" t thème • F5 actualiser"),
            Line::from(""),
            Line::from(Span::styled(
                " BASE DE DONNÉES",
                Style::default().fg(colors.accent),
            )),
            Line::from(format!(" {}", app.db_status)),
            Line::from(Span::styled(
                " b configurer / migrer",
                Style::default().fg(colors.muted),
            )),
        ];
        frame.render_widget(
            Paragraph::new(status)
                .wrap(Wrap { trim: false })
                .block(panel(" État ", false, colors)),
            columns[2],
        );
    }
    let height = layout[2].height.saturating_sub(2) as usize;
    let end = app.logs.len().saturating_sub(app.log_scroll);
    let start = end.saturating_sub(height);
    let logs: Vec<Line> = app
        .logs
        .iter()
        .skip(start)
        .take(end - start)
        .map(|line| Line::from(line.clone()))
        .collect();
    let log_title = if wide {
        " Logs ".into()
    } else {
        format!(" Logs · API : {} ", app.api_status)
    };
    frame.render_widget(
        Paragraph::new(logs).block(panel(log_title, app.focus == Focus::Logs, colors)),
        layout[2],
    );
    frame.render_widget(
        Paragraph::new(app.notice.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(colors.accent))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(colors.border)),
            ),
        layout[3],
    );
    if let Some(dialog) = &app.dialog {
        draw_dialog(frame, dialog, colors);
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(4));
    let height = height.min(area.height.saturating_sub(4));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
fn draw_dialog(frame: &mut Frame, dialog: &forms::Dialog, colors: Palette) {
    use forms::Dialog;
    let height = match dialog {
        Dialog::Input { .. } => 7,
        Dialog::Select {
            description, items, ..
        } => (description.len() + items.len() + 5) as u16,
        Dialog::Options { items, .. } => items.len() as u16 + 5,
        Dialog::Help => 22,
    };
    let area = centered(frame.area(), 78, height);
    frame.render_widget(Clear, area);
    match dialog {
        Dialog::Input { title, value, kind } => {
            let block = panel(format!(" {title} "), true, colors);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let width = inner.width.saturating_sub(2) as usize;
            let masked;
            let value = if matches!(kind, forms::InputKind::DbPassword) {
                masked = "•".repeat(value.chars().count());
                &masked
            } else {
                value
            };
            let count = value.chars().count();
            let shown: String = value.chars().skip(count.saturating_sub(width)).collect();
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(format!(" {shown}")),
                    Line::from(""),
                    Line::from(Span::styled(
                        " Entrée : valider • Ctrl+U : effacer • Échap : annuler",
                        Style::default().fg(colors.muted),
                    )),
                ])
                .style(Style::default().fg(colors.foreground).bg(colors.panel)),
                inner,
            );
            frame.set_cursor_position((
                inner.x + 1 + shown.chars().count().min(width) as u16,
                inner.y + 1,
            ));
        }
        Dialog::Select {
            title,
            description,
            items,
            selected,
            ..
        } => {
            let block = panel(format!(" {title} "), true, colors);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let description_height = (description.len() as u16).min(inner.height.saturating_sub(3));
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(description_height),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(inner);
            frame.render_widget(
                Paragraph::new(description.join("\n")).style(Style::default().fg(colors.muted)),
                rows[0],
            );
            let mut state = ListState::default().with_selected(Some(*selected));
            frame.render_stateful_widget(
                List::new(
                    items
                        .iter()
                        .map(|s| ListItem::new(s.clone()))
                        .collect::<Vec<_>>(),
                )
                .style(Style::default().fg(colors.foreground))
                .highlight_style(Style::default().bg(colors.selected).fg(colors.accent))
                .highlight_symbol("› "),
                rows[1],
                &mut state,
            );
            frame.render_widget(
                Paragraph::new("↑ ↓ ou j/k • Entrée : valider • Échap : annuler")
                    .style(Style::default().fg(colors.muted)),
                rows[2],
            );
        }
        Dialog::Options {
            items, selected, ..
        } => {
            let block = panel(" Options du champ ", true, colors);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
            let mut state = ListState::default().with_selected(Some(*selected));
            frame.render_stateful_widget(
                List::new(
                    items
                        .iter()
                        .map(|(name, checked)| {
                            ListItem::new(format!("[{}] {name}", if *checked { "×" } else { " " }))
                        })
                        .collect::<Vec<_>>(),
                )
                .style(Style::default().fg(colors.foreground))
                .highlight_style(Style::default().fg(colors.accent).bg(colors.selected))
                .highlight_symbol("› "),
                rows[0],
                &mut state,
            );
            frame.render_widget(
                Paragraph::new("Espace : cocher • Entrée : valider\nÉchap : annuler la ressource")
                    .style(Style::default().fg(colors.muted)),
                rows[1],
            );
        }
        Dialog::Help => {
            frame.render_widget(Paragraph::new("NAVIGATION\n  Tab / Shift+Tab   changer de panneau\n  ↑ ↓ ou j/k        naviguer / défiler\n  Entrée            ouvrir un fichier / dossier / ressource\n  PageUp / PageDown défiler plus vite\n  h / l             défiler horizontalement dans le code\n\nACTIONS\n  n créer • o ouvrir • a ajouter un modèle • b base de données\n  r lancer •  s arrêter •  c vérifier • e éditeur\n  Espace   palette d’actions   • t changer le thème\n  F5 actualiser • q quitter (arrête l’API lancée ici)\n\nCOMMANDES AVEC :\n  new, open [dossier], add, run, stop, check, test\n  db, migrate, doctor, edit, refresh, theme, help, quit\n\nLe code est en lecture seule. VISUAL/EDITOR choisit l’éditeur.\nAppuie sur une touche pour fermer.")
                .style(Style::default().fg(colors.foreground)).block(panel(" Aide Miblo ", true, colors)).wrap(Wrap { trim: false }), area);
        }
    }
}
