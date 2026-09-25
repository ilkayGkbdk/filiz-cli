use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::history::SeriesKey;
use crate::state::{DiskStats, InterfaceStats, SystemState};
use crate::ui::components::card::resource_card;
use crate::ui::format::{bytes, percent, rate};
use crate::ui::state::{LayoutDensity, PanelId};
use crate::ui::RenderCx;

use super::processes::process_table;
use super::WorkspaceView;

pub struct Overview;

impl WorkspaceView for Overview {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        let (resource, detail) = match cx.app.ui.density {
            LayoutDensity::Compact => (7, 3),
            LayoutDensity::Spacious => (15, 8),
            LayoutDensity::Balanced => match cx.frame_height {
                28.. => (12, 6),
                20..=27 => (8, 3),
                17..=19 => (6, 3),
                _ => (2, 0),
            },
        };
        let resource = if cx.visible(PanelId::Resources) {
            resource
        } else {
            0
        };
        let detail = if cx.visible(PanelId::Details) {
            detail
        } else {
            0
        };
        let middle = if cx.visible(PanelId::Processes) {
            Constraint::Min(0)
        } else {
            Constraint::Length(0)
        };
        let [top, center, bottom] = Layout::vertical([
            Constraint::Length(resource),
            middle,
            Constraint::Length(detail),
        ])
        .areas(area);
        resources(frame, top, cx);
        if cx.visible(PanelId::Processes) {
            process_table(frame, center, cx);
        }
        details(frame, bottom, cx);
    }
}

pub(crate) fn resources(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let app = cx.app;
    let palette = cx.palette;
    let cpu = app.state.cpu.usage;
    let memory = app.state.memory.usage_percent();
    let disk = app.state.primary_disk().and_then(DiskStats::usage_percent);
    let (rx, tx) = total_rates(&app.state);
    let network_history = app
        .selected_interface()
        .map(|i| app.history.series(&SeriesKey::NetRx(i.name.clone())))
        .unwrap_or_default();
    let cards = [
        (
            "CPU",
            percent(cpu),
            cpu_detail(&app.state),
            cpu,
            app.history.series(&SeriesKey::Cpu),
        ),
        (
            "MEMORY",
            percent(memory),
            memory_detail(&app.state),
            memory,
            app.history.series(&SeriesKey::Memory),
        ),
        (
            "DISK",
            percent(disk),
            disk_detail(&app.state),
            disk,
            app.history.series(&SeriesKey::Disk),
        ),
        (
            "NETWORK",
            rx.map(|value| format!("↓ {}", rate(value)))
                .unwrap_or_else(|| "↓ N/A".into()),
            tx.map(|value| format!("↑ {}", rate(value)))
                .unwrap_or_else(|| "↑ N/A".into()),
            None,
            network_history,
        ),
    ];
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    for (row, pair) in cards.chunks(2).enumerate() {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[row]);
        for (column, card) in pair.iter().enumerate() {
            resource_card(
                frame,
                columns[column],
                card,
                cx.focused(PanelId::Resources),
                &palette,
            );
        }
    }
}

pub(crate) fn details(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    use ratatui::{
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Paragraph, Wrap},
    };

    use crate::ui::components::table::border_for;

    if area.height == 0 || area.width == 0 {
        return;
    }
    let app = cx.app;
    let palette = cx.palette;
    let selected = app.selected_process();
    let warning = app.state.issues().next();
    let mut lines = Vec::new();
    if let Some(notice) = &app.notice {
        lines.push(Line::from(Span::styled(
            format!(" EVENT  {notice}"),
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        )));
    } else if let Some((source, message)) = warning {
        lines.push(Line::from(Span::styled(
            format!(" WARNING  {source:?}: {message}"),
            Style::default()
                .fg(palette.warn)
                .add_modifier(Modifier::BOLD),
        )));
    }
    if let Some(process) = selected {
        lines.extend([
            Line::from(vec![
                Span::styled(" PROCESS  ", Style::default().fg(palette.text_muted)),
                Span::styled(
                    process.name,
                    Style::default()
                        .fg(palette.text)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  PID {}", process.identity.pid),
                    Style::default().fg(palette.accent),
                ),
            ]),
            Line::from(vec![
                Span::styled(" COMMAND  ", Style::default().fg(palette.text_muted)),
                Span::styled(process.command, Style::default().fg(palette.text)),
            ]),
            Line::from(vec![
                Span::styled(" USER  ", Style::default().fg(palette.text_muted)),
                Span::raw(process.user.unwrap_or_else(|| "N/A".into())),
                Span::styled("   STATE  ", Style::default().fg(palette.text_muted)),
                Span::raw(process.status.unwrap_or_else(|| "N/A".into())),
            ]),
        ]);
    } else {
        lines.push(Line::from(" No process selected"));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" DETAILS / EVENTS ")
        .title_style(Style::default().fg(palette.accent))
        .border_style(border_for(cx, PanelId::Details))
        .style(Style::default().bg(palette.surface).fg(palette.text));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn cpu_detail(state: &SystemState) -> String {
    let cpu = &state.cpu;
    let cores = cpu
        .cores
        .map(|cores| format!("{cores} CORES"))
        .unwrap_or_else(|| "CORES N/A".into());
    let idle = cpu
        .idle()
        .map(|idle| format!("IDLE {idle:.0}%"))
        .unwrap_or_else(|| "IDLE N/A".into());
    let breakdown = match (cpu.user, cpu.system) {
        (Some(user), Some(system)) => format!("USER {user:.0}% · SYS {system:.0}%"),
        _ => "USER N/A · SYS N/A".into(),
    };
    let load = cpu
        .load
        .map(|load| format!("LOAD {:.2}", load[0]))
        .unwrap_or_else(|| "LOAD N/A".into());
    let temperature = cpu
        .temperature
        .map(|t| format!("TEMP {t:.0}°C"))
        .unwrap_or_else(|| "TEMP N/A".into());
    format!("{cores} · {idle} · {breakdown} · {load} · {temperature}")
}

fn opt_bytes(value: Option<u64>) -> String {
    value.map(bytes).unwrap_or_else(|| "N/A".into())
}

fn memory_detail(state: &SystemState) -> String {
    let memory = &state.memory;
    format!(
        "USED {} · FREE {} · AVAIL {} · SWAP {}",
        opt_bytes(memory.used),
        opt_bytes(memory.free),
        opt_bytes(memory.available),
        opt_bytes(memory.swap_used)
    )
}

fn disk_detail(state: &SystemState) -> String {
    let disk = state.primary_disk();
    format!(
        "USED {} · FREE {} · TOTAL {}",
        opt_bytes(disk.map(|d| d.used)),
        opt_bytes(disk.map(|d| d.free)),
        opt_bytes(disk.map(|d| d.total))
    )
}

fn total_rates(state: &SystemState) -> (Option<f64>, Option<f64>) {
    let sum = |pick: fn(&InterfaceStats) -> Option<f64>| {
        let values: Vec<f64> = state.interfaces.iter().filter_map(pick).collect();
        (!values.is_empty()).then(|| values.iter().sum())
    };
    (sum(|i| i.rx_rate), sum(|i| i.tx_rate))
}
