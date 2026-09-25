use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::input::action::Action;
use crate::model::ActionKind;
use crate::ui::format::bytes;
use crate::ui::hit::HitTarget;
use crate::ui::RenderCx;

pub fn detail(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    let Some(process) = cx.app.selected_process() else {
        return;
    };
    let popup = centered(area, 70, 11);
    if popup.width < 4 || popup.height < 4 {
        return;
    }
    cx.out.hits.push(area, HitTarget::Blocker);
    let palette = cx.palette;
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(format!(
            "  {}  •  PID {}",
            process.name, process.identity.pid
        )),
        Line::from(""),
        Line::from(format!(
            "  CPU      {}",
            process
                .cpu_percent
                .map(|v| format!("{v:.1}%"))
                .unwrap_or_else(|| "N/A".into())
        )),
        Line::from(format!(
            "  MEMORY   {}",
            process
                .memory_bytes
                .map(bytes)
                .unwrap_or_else(|| "N/A".into())
        )),
        Line::from(format!(
            "  USER     {}",
            process.user.unwrap_or_else(|| "N/A".into())
        )),
        Line::from(format!(
            "  STATE    {}",
            process.status.unwrap_or_else(|| "N/A".into())
        )),
        Line::from(format!("  COMMAND  {}", process.command)),
        Line::from(""),
        Line::from(Span::styled(
            "  ESC CLOSE   K TERMINATE   SHIFT+K KILL",
            Style::default().fg(palette.accent),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" PROCESS DETAIL ")
        .border_style(Style::default().fg(palette.accent))
        .style(Style::default().fg(palette.text).bg(palette.surface));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup,
    );
}

pub fn confirmation(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    let app = cx.app;
    let Some(pending) = app.pending_action else {
        return;
    };
    let popup = centered(area, 56, 9);
    if popup.width < 4 || popup.height < 4 {
        return;
    }
    cx.out.hits.push(area, HitTarget::Blocker);
    let palette = cx.palette;
    let name = app
        .state
        .processes
        .iter()
        .find(|p| p.identity == pending.identity())
        .map(|process| process.name.as_str())
        .unwrap_or("N/A");
    let action = match pending.kind() {
        ActionKind::Terminate => "TERMINATE",
        ActionKind::Kill => "KILL",
    };
    frame.render_widget(Clear, popup);
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{action} PROCESS?"),
            Style::default()
                .fg(palette.danger)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("{name}  /  PID {}", pending.identity().pid)),
        Line::from(""),
        Line::from(Span::styled(
            "Y  CONFIRM              N / ESC  CANCEL",
            Style::default().fg(palette.warn),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" CONFIRM ACTION ")
        .title_style(
            Style::default()
                .fg(palette.danger)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(palette.danger))
        .style(Style::default().fg(palette.text).bg(palette.surface));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center),
        popup,
    );
    let line = Rect {
        x: popup.x + 1,
        y: popup.y + 6,
        width: popup.width.saturating_sub(2),
        height: 1,
    };
    let half = line.width / 2;
    cx.out.hits.push(
        Rect {
            width: half,
            ..line
        },
        HitTarget::Button(Action::Confirm),
    );
    cx.out.hits.push(
        Rect {
            x: line.x + half,
            width: line.width - half,
            ..line
        },
        HitTarget::Button(Action::Cancel),
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}
