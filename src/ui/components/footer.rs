use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::input::action::Action;
use crate::input::keymap;
use crate::model::AppMode;
use crate::ui::hit::HitTarget;
use crate::ui::RenderCx;

pub fn render(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let app = cx.app;
    let palette = cx.palette;
    let max_width = area.width as usize;
    let mut lines: Vec<Vec<Span>> = vec![Vec::new()];
    let mut width = 0;
    if app.mode == AppMode::Filtering {
        let prompt = format!(" {}_  ", app.filter);
        width = 8 + prompt.chars().count();
        lines[0].push(Span::styled(
            " FILTER ",
            Style::default().fg(palette.accent_fg).bg(palette.accent),
        ));
        lines[0].push(Span::styled(prompt, Style::default().fg(palette.text)));
    }
    for binding in keymap::hints(&app.contexts()) {
        let hint = binding.hint.unwrap_or_default();
        let needed = binding.label.chars().count() + hint.chars().count() + 4;
        if width + needed > max_width {
            if lines.len() == area.height as usize {
                break;
            }
            lines.push(Vec::new());
            width = 0;
        }
        let x = area.x + width as u16;
        let y = area.y + (lines.len() - 1) as u16;
        // Modals (`ConfirmingAction`, `ProcessDetail`) draw a full-screen `Blocker`
        // after the footer and take over the same key bindings shown here (e.g.
        // Y/CONFIRM, N/CANCEL), so a footer hint would never be reachable by mouse
        // in those modes; skip registering it there to avoid a duplicate,
        // unreachable `HitTarget::Button` shadowing the modal's own button.
        if app.mode == AppMode::Dashboard {
            cx.out.hits.push(
                Rect {
                    x,
                    y,
                    width: needed as u16,
                    height: 1,
                },
                HitTarget::Button(binding.action),
            );
        }
        width += needed;
        let key_color = if matches!(binding.action, Action::Terminate | Action::Kill) {
            palette.warn
        } else {
            palette.accent
        };
        let line = lines.last_mut().expect("footer line");
        line.push(Span::styled(
            format!("  {} ", binding.label),
            Style::default().fg(key_color).add_modifier(Modifier::BOLD),
        ));
        line.push(Span::styled(
            format!("{hint} "),
            Style::default().fg(palette.text_muted),
        ));
    }
    let lines: Vec<Line> = lines.into_iter().map(Line::from).collect();
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.surface)),
        area,
    );
}
