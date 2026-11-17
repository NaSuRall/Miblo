use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::Paragraph,
};
use std::time::{Duration, Instant};

const LETTERS: [[&str; 5]; 5] = [
    ["█     █", "██   ██", "█ █ █ █", "█  █  █", "█     █"],
    ["█████", "  █  ", "  █  ", "  █  ", "█████"],
    ["████ ", "█   █", "████ ", "█   █", "████ "],
    ["█    ", "█    ", "█    ", "█    ", "█████"],
    [" ███ ", "█   █", "█   █", "█   █", " ███ "],
];

/// A brief welcome, only in the interactive workspace. Any key skips it.
pub(super) fn show(terminal: &mut DefaultTerminal) -> Result<bool> {
    if std::env::var_os("MIBLO_NO_ANIMATION").is_some() {
        return Ok(true);
    }
    let start = Instant::now();
    let duration = Duration::from_millis(900);
    while start.elapsed() < duration {
        let count = ((start.elapsed().as_millis() / 130) as usize + 1).min(5);
        terminal.draw(|frame| draw(frame, count))?;
        if event::poll(Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
            && key.kind != KeyEventKind::Release
        {
            return Ok(
                !(key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
            );
        }
    }
    Ok(true)
}

fn draw(frame: &mut Frame, count: usize) {
    let area = frame.area();
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(Color::Rgb(10, 15, 28))),
        area,
    );
    let large = area.width >= 41 && area.height >= 12;
    let width = if large { 35 } else { 5 };
    let height = if large { 5 } else { 1 };
    let logo = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height + 4) / 2,
        width.min(area.width),
        height.min(area.height),
    );
    let lines: Vec<Line> = if large {
        (0..5)
            .map(|row| {
                Line::from(
                    LETTERS[..count]
                        .iter()
                        .map(|letter| letter[row])
                        .collect::<Vec<_>>()
                        .join("  "),
                )
            })
            .collect()
    } else {
        vec![Line::from("MIBLO"[..count].to_owned())]
    };
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::Rgb(70, 150, 255))),
        logo,
    );
    let subtitle_y = logo.y.saturating_add(height + 2);
    if subtitle_y < area.bottom() {
        frame.render_widget(
            Paragraph::new("Votre API prend forme.")
                .centered()
                .style(Style::default().fg(Color::Rgb(140, 165, 200))),
            Rect::new(area.x, subtitle_y, area.width, 1),
        );
    }
    if area.height > 12 {
        frame.render_widget(
            Paragraph::new("Une touche pour passer")
                .centered()
                .style(Style::default().fg(Color::DarkGray)),
            Rect::new(area.x, area.bottom() - 2, area.width, 1),
        );
    }
}
