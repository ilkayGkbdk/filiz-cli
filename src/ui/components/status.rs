use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::format::uptime as format_uptime;
use crate::ui::hit::HitTarget;
use crate::ui::state::Workspace;
use crate::ui::RenderCx;

pub fn render(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    let app = cx.app;
    let palette = cx.palette;
    let failing = app.state.has_failures();
    let health = if !failing {
        "SYSTEM NORMAL"
    } else {
        "CHECK METRICS"
    };
    let health_color = if !failing { palette.ok } else { palette.warn };
    let uptime = app
        .state
        .uptime
        .map(|d| format_uptime(d.as_secs()))
        .unwrap_or_else(|| "N/A".into());
    let battery = app
        .state
        .battery
        .as_ref()
        .map(|b| format!("{:.0}%", b.percent))
        .unwrap_or_else(|| "N/A".into());
    let temperature = app
        .state
        .cpu
        .temperature
        .map(|t| format!("{t:.0}°C"))
        .unwrap_or_else(|| "N/A".into());
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "  FILIZ  ",
                Style::default()
                    .fg(palette.bg)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  SYSTEM MONITOR  /  ",
                Style::default().fg(palette.text_muted),
            ),
            Span::styled(
                health,
                Style::default()
                    .fg(health_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  /  {}", app.ui.workspace.label()),
                Style::default().fg(palette.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("  UPTIME ", Style::default().fg(palette.text_muted)),
            Span::styled(uptime, Style::default().fg(palette.text)),
            Span::styled("   BATTERY ", Style::default().fg(palette.text_muted)),
            Span::styled(battery, Style::default().fg(palette.text)),
            Span::styled("   TEMP ", Style::default().fg(palette.text_muted)),
            Span::styled(temperature, Style::default().fg(palette.text)),
        ]),
    ];
    if area.height >= 3 {
        let mut x = area.x;
        let mut tabs = Vec::new();
        for (index, workspace) in Workspace::ALL.iter().enumerate() {
            let text = format!("  {} {}  ", index + 1, workspace.label());
            let width = text.chars().count() as u16;
            cx.out.hits.push(
                Rect {
                    x,
                    y: area.y + 2,
                    width: width.min(area.right().saturating_sub(x)),
                    height: 1,
                },
                HitTarget::Tab(*workspace),
            );
            x = x.saturating_add(width);
            let style = if *workspace == app.ui.workspace {
                Style::default()
                    .fg(palette.bg)
                    .bg(palette.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_muted)
            };
            tabs.push(Span::styled(text, style));
        }
        lines.push(Line::from(tabs));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.surface)),
        area,
    );
}
