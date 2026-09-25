use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::hit::HitTarget;
use crate::ui::state::PanelId;
use crate::ui::RenderCx;

use super::WorkspaceView;

pub struct More;

impl WorkspaceView for More {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        more(frame, area, cx);
    }
}

fn more(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    cx.out.hits.push(area, HitTarget::Panel(PanelId::Settings));
    let app = cx.app;
    let palette = cx.palette;
    let temperature = app
        .state
        .cpu
        .temperature
        .map(|value| format!("{value:.0}°C"))
        .unwrap_or_else(|| "N/A".into());
    let battery = app
        .state
        .battery
        .as_ref()
        .map(|b| format!("{:.0}%", b.percent))
        .unwrap_or_else(|| "N/A".into());
    let mut lines = vec![
        Line::from(Span::styled(
            "  FILIZ CONTROL CENTER",
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("  THEME    {}", app.ui.theme.label())),
        Line::from(format!("  DENSITY  {:?}", app.ui.density)),
        Line::from(format!("  SENSORS  TEMP {temperature} · BATTERY {battery}")),
        Line::from("  ABOUT    Filiz macOS monitor · MIT License"),
        Line::from(""),
        Line::from(Span::styled(
            "  for betül, with love ♡",
            Style::default().fg(palette.warn),
        )),
    ];
    if app.ui.menu_open {
        lines.extend([
            Line::from(""),
            Line::from(Span::styled(
                "  MENU OPEN  [L] density  [T] theme  [Esc] close",
                Style::default().fg(palette.ok).add_modifier(Modifier::BOLD),
            )),
            Line::from("  Layout: compact / balanced / spacious"),
            Line::from("  Theme: Forest / Amber / Mono / Solarized"),
        ]);
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" MORE / SETTINGS ")
        .title_style(Style::default().fg(palette.accent))
        .border_style(Style::default().fg(palette.border))
        .style(Style::default().bg(palette.surface));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(palette.text)),
        area,
    );
}
