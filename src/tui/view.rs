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
        .title_style(Style::default().fg(if focused { colors.accent } else { colors.muted }))
        .style(Style::default().fg(colors.foreground).bg(colors.panel))
}

fn tone_color(tone: status::Tone, light: bool) -> Color {
    match (tone, light) {
        (status::Tone::Good, false) => Color::Rgb(101, 219, 147),
        (status::Tone::Good, true) => Color::Rgb(24, 120, 64),
        (status::Tone::Warning, false) => Color::Rgb(255, 185, 86),
        (status::Tone::Warning, true) => Color::Rgb(155, 85, 0),
        (status::Tone::Error, false) => Color::Rgb(255, 119, 133),
        (status::Tone::Error, true) => Color::Rgb(184, 35, 57),
        (status::Tone::Neutral, _) => Color::Rgb(131, 145, 170),
    }
}

fn badge(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    badge: status::Badge,
    extra: &str,
    colors: Palette,
    light: bool,
) {
    let lines = vec![
        Line::from(Span::styled(
            format!(" ● {}", badge.title),
            Style::default()
                .fg(tone_color(badge.tone, light))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(" {}", clean(&badge.detail))),
        Line::from(Span::styled(
            format!(" {}", clean(extra)),
            Style::default().fg(colors.muted),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(panel(title.to_owned(), false, colors)),
        area,
    );
}

pub(super) fn draw(frame: &mut Frame, app: &mut App) {
    let colors = palette(app.theme);
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(colors.background).fg(colors.foreground)),
        area,
    );
    app.code_visible = app.show_code && area.height >= 30 && app.project.is_some();
    if !app.code_visible && app.focus == Focus::Code {
        app.focus = Focus::Routes;
    }
    if area.width < 70 || area.height < 20 {
        frame.render_widget(
            Paragraph::new("MIBLO\nAgrandis le terminal à au moins 70 × 20.\nq pour quitter")
                .style(Style::default().fg(colors.accent)),
            area,
        );
        return;
    }
    let layout = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(5),
        Constraint::Min(6),
        Constraint::Length(if area.height >= 30 { 6 } else { 4 }),
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
        Span::raw(format!("  {project}  ")),
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
    frame.render_widget(Paragraph::new(title), layout[0]);
    let statuses = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[1]);
    badge(
        frame,
        statuses[0],
        " État API · r lancer / s arrêter ",
        app.api_badge(),
        &app.base_url,
        colors,
        app.theme == 2,
    );
    let db = if app.project.is_none() {
        status::Badge::new(
            status::Tone::Neutral,
            "Aucune base sélectionnée",
            "Crée ou ouvre une API",
        )
    } else {
        status::database_badge(&app.monitor.database, app.db_job.is_some())
    };
    badge(
        frame,
        statuses[1],
        " État BDD · b configurer / migrer ",
        db,
        &app.status,
        colors,
        app.theme == 2,
    );
    let columns = Layout::horizontal([
        Constraint::Length(if area.width >= 110 { 27 } else { 23 }),
        Constraint::Min(30),
    ])
    .split(layout[2]);
    let sidebar = Layout::vertical([Constraint::Length(6), Constraint::Min(3)]).split(columns[0]);
    let resources: Vec<_> = app
        .project
        .as_ref()
        .map(|p| {
            p.resources
                .iter()
                .map(|r| {
                    ListItem::new(vec![
                        Line::from(r.name.clone()),
                        Line::from(Span::styled(
                            r.path.clone(),
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
    let mut file_state = ListState::default().with_selected(if files.is_empty() {
        None
    } else {
        Some(app.selected)
    });
    frame.render_stateful_widget(
        List::new(files)
            .block(panel(
                " Fichiers · e éditeur ",
                app.focus == Focus::Files,
                colors,
            ))
            .highlight_style(Style::default().bg(colors.selected))
            .highlight_symbol("›"),
        sidebar[1],
        &mut file_state,
    );
    if app.project.is_none() {
        frame.render_widget(Paragraph::new("\n  Ton API, au même endroit.\n\n  n créer une API · o ouvrir un projet\n  a ajouter un modèle / une table\n  b configurer la base / les migrations\n\n  Espace : actions · ? : aide\n  Flèches / j/k : naviguer · Tab : panneau suivant").wrap(Wrap { trim: false }).block(panel(" Atelier API ", false, colors)), columns[1]);
    } else {
        let work = Layout::vertical([
            Constraint::Min(6),
            Constraint::Length(if app.code_visible { 6 } else { 0 }),
        ])
        .split(columns[1]);
        let routes_area = if area.width >= 110 {
            Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(work[0])
        } else {
            Layout::vertical([Constraint::Length(5), Constraint::Min(3)]).split(work[0])
        };
        let routes: Vec<_> = app
            .routes
            .iter()
            .map(|r| ListItem::new(format!("{:<6} {}", r.method, r.path)))
            .collect();
        let mut route_state = ListState::default().with_selected(Some(app.route_selected));
        frame.render_stateful_widget(
            List::new(routes)
                .block(panel(
                    " Routes · x tester · f libre ",
                    app.focus == Focus::Routes,
                    colors,
                ))
                .highlight_style(Style::default().fg(colors.accent).bg(colors.selected))
                .highlight_symbol("› "),
            routes_area[0],
            &mut route_state,
        );
        let response_title = if app.tester.pending {
            Line::from(" Réponse · requête en cours… ")
        } else if let Some(reply) = &app.tester.reply {
            let (tone, label) = match reply.status {
                Some(code) => (
                    if (200..300).contains(&code) {
                        status::Tone::Good
                    } else if code < 400 {
                        status::Tone::Warning
                    } else {
                        status::Tone::Error
                    },
                    format!("HTTP {code}"),
                ),
                None => (status::Tone::Error, "Erreur réseau".into()),
            };
            Line::from(Span::styled(
                format!(" Réponse · ● {label} · {} ms ", reply.elapsed_ms),
                Style::default().fg(tone_color(tone, app.theme == 2)),
            ))
        } else {
            Line::from(" Réponse HTTP ")
        };
        let mut response = Vec::new();
        if let Some(reply) = &app.tester.reply {
            response.push(Line::from(Span::styled(
                clean(&format!(
                    "{} {} · {}",
                    reply.method, reply.path, reply.content_type
                )),
                Style::default().fg(colors.muted),
            )));
            response.extend(
                app.tester
                    .lines
                    .iter()
                    .skip(app.tester.scroll)
                    .take(routes_area[1].height.saturating_sub(3) as usize)
                    .map(|line| Line::from(line.clone())),
            );
        } else {
            response.push(Line::from(if app.tester.pending {
                "Envoi en cours · délai maximum 10 s"
            } else {
                "Sélectionne une route puis x ou Entrée."
            }));
            response.push(Line::from("f : choisir méthode, chemin et JSON"));
            response.push(Line::from("Les écritures demandent confirmation."));
        }
        frame.render_widget(
            Paragraph::new(response).block(panel(
                response_title,
                app.focus == Focus::Response,
                colors,
            )),
            routes_area[1],
        );
        if app.code_visible {
            let filename = app
                .file
                .as_ref()
                .map_or(String::new(), |p| p.display().to_string());
            let lines: Vec<_> = app
                .code
                .iter()
                .enumerate()
                .skip(app.code_scroll)
                .take(4)
                .map(|(i, line)| {
                    Line::from(vec![
                        Span::styled(
                            format!("{:>4} │ ", i + 1),
                            Style::default().fg(colors.muted),
                        ),
                        Span::raw(
                            line.chars()
                                .skip(app.code_horizontal as usize)
                                .collect::<String>(),
                        ),
                    ])
                })
                .collect();
            frame.render_widget(
                Paragraph::new(lines).block(panel(
                    format!(" Aperçu · {filename} · e éditeur · v masquer "),
                    app.focus == Focus::Code,
                    colors,
                )),
                work[1],
            );
        }
    }
    let height = layout[3].height.saturating_sub(2) as usize;
    let end = app.logs.len().saturating_sub(app.log_scroll);
    let logs: Vec<_> = app
        .logs
        .iter()
        .skip(end.saturating_sub(height))
        .take(height)
        .map(|line| Line::from(line.clone()))
        .collect();
    let action = app
        .job
        .as_ref()
        .map_or("Aucune action en cours", |p| p.label.as_str());
    let count = app.project.as_ref().map_or(0, |p| p.resources.len());
    frame.render_widget(
        Paragraph::new(logs).block(panel(
            format!(" Logs · {count} ressource(s) · {action} "),
            app.focus == Focus::Logs,
            colors,
        )),
        layout[3],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(app.notice.as_str()),
            Line::from(Span::styled(
                "Tab panneaux · x tester · f requête · b BDD · e éditeur · v code · ? aide",
                Style::default().fg(colors.muted),
            )),
        ])
        .style(Style::default().fg(colors.accent))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(colors.border)),
        ),
        layout[4],
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
        Dialog::Help => 24,
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
            frame.render_widget(Paragraph::new("NAVIGATION\n  Tab / Shift+Tab   changer de panneau\n  ↑ ↓ ou j/k        naviguer / défiler\n  Entrée            ouvrir un fichier / dossier / ressource\n  PageUp / PageDown défiler plus vite\n  h / l             défiler horizontalement dans le code\n\nACTIONS\n  n créer • o ouvrir • a ajouter un modèle • b base de données\n  r lancer • s arrêter • c vérifier • e éditeur\n  x tester la route • f requête HTTP libre • v masquer le code\n  Espace   palette d’actions   • t changer le thème\n  F5 actualiser • q quitter (arrête l’API lancée ici)\n\nCOMMANDES AVEC :\n  new, open [dossier], add, run, stop, check, test\n  db, migrate, routes, request, code, edit, refresh, help, quit\n\nLe code est en lecture seule. VISUAL/EDITOR choisit l’éditeur.\nAppuie sur une touche pour fermer.")
                .style(Style::default().fg(colors.foreground)).block(panel(" Aide Miblo ", true, colors)).wrap(Wrap { trim: false }), area);
        }
    }
}
