use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Sparkline, Table, TableState, Wrap},
    Frame,
};

use crate::app::{App, Panel};
use crate::model::{ActionKind, AppMode, ProcessInfo, SortMode, SystemSnapshot};

use super::theme;

pub fn status(frame: &mut Frame, area: Rect, app: &App) {
    let snapshot = app.snapshot.as_ref();
    let warnings = snapshot.map_or(0, |snapshot| snapshot.warnings.len());
    let health = if warnings == 0 {
        "SYSTEM NORMAL"
    } else {
        "CHECK METRICS"
    };
    let health_color = if warnings == 0 {
        theme::GREEN
    } else {
        theme::YELLOW
    };
    let uptime = snapshot
        .and_then(|snapshot| value(snapshot, "uptime"))
        .map(|seconds| format_uptime(seconds as u64))
        .unwrap_or_else(|| "N/A".into());
    let battery = snapshot
        .and_then(|snapshot| value(snapshot, "battery.percent"))
        .map(|percent| format!("{percent:.0}%"))
        .unwrap_or_else(|| "N/A".into());
    let temperature = snapshot
        .and_then(|snapshot| value(snapshot, "temperature.celsius"))
        .map(|degrees| format!("{degrees:.0}°C"))
        .unwrap_or_else(|| "N/A".into());
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "  FILIZ  ",
                Style::default()
                    .fg(theme::BACKGROUND)
                    .bg(theme::OLIVE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  SYSTEM MONITOR  /  ", Style::default().fg(theme::MUTED)),
            Span::styled(
                health,
                Style::default()
                    .fg(health_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  UPTIME ", Style::default().fg(theme::MUTED)),
            Span::styled(uptime, Style::default().fg(theme::TEXT)),
            Span::styled("   BATTERY ", Style::default().fg(theme::MUTED)),
            Span::styled(battery, Style::default().fg(theme::TEXT)),
            Span::styled("   TEMP ", Style::default().fg(theme::MUTED)),
            Span::styled(temperature, Style::default().fg(theme::TEXT)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme::PANEL)),
        area,
    );
}

pub fn resources(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let snapshot = app.snapshot.as_ref();
    let cpu = snapshot.and_then(|s| value(s, "cpu.usage"));
    let memory = snapshot.and_then(|s| value(s, "memory.usage"));
    let disk = snapshot.and_then(disk_usage);
    let (rx, tx) = snapshot.map(network_rates).unwrap_or((None, None));
    let cards = [
        (
            "CPU",
            percent(cpu),
            snapshot
                .and_then(|s| value(s, "cpu.cores"))
                .map(|cores| format!("{cores:.0} CORES"))
                .unwrap_or_else(|| "CORES N/A".into()),
            cpu,
            &app.histories[0],
        ),
        (
            "MEMORY",
            percent(memory),
            memory_capacity(snapshot),
            memory,
            &app.histories[1],
        ),
        (
            "DISK",
            percent(disk),
            disk_capacity(snapshot),
            disk,
            &app.histories[2],
        ),
        (
            "NETWORK",
            rx.map(|value| format!("↓ {}", rate(value)))
                .unwrap_or_else(|| "↓ N/A".into()),
            tx.map(|value| format!("↑ {}", rate(value)))
                .unwrap_or_else(|| "↑ N/A".into()),
            None,
            &app.histories[3],
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
            resource_card(frame, columns[column], card, app.focus == Panel::Resources);
        }
    }
}

fn resource_card(
    frame: &mut Frame,
    area: Rect,
    card: &(&str, String, String, Option<f64>, &Vec<u64>),
    focused: bool,
) {
    let (title, main, secondary, usage, history) = card;
    if area.width < 3 || area.height < 2 {
        return;
    }
    let border = if focused { theme::OLIVE } else { theme::BORDER };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme::PANEL).fg(theme::TEXT))
        .border_style(Style::default().fg(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let color = usage.map(usage_color).unwrap_or(theme::OLIVE);
    frame.render_widget(
        Paragraph::new(main.to_owned()).style(
            Style::default()
                .fg(color)
                .bg(theme::PANEL)
                .add_modifier(Modifier::BOLD),
        ),
        Rect { height: 1, ..inner },
    );
    if inner.height > 1 {
        frame.render_widget(
            Paragraph::new(secondary.to_owned())
                .style(Style::default().fg(theme::MUTED).bg(theme::PANEL)),
            Rect {
                y: inner.y + 1,
                height: 1,
                ..inner
            },
        );
    }
    if inner.height > 2 && !history.is_empty() {
        let spark_area = Rect {
            y: inner.y + 2,
            height: inner.height - 2,
            ..inner
        };
        frame.render_widget(
            Sparkline::default()
                .data(*history)
                .max(if usage.is_some() {
                    100
                } else {
                    history.iter().copied().max().unwrap_or(1).max(1)
                })
                .style(Style::default().fg(color).bg(theme::PANEL)),
            spark_area,
        );
    }
}

pub fn processes(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let processes = app.visible_processes();
    let sort = match app.sort {
        SortMode::Cpu => "CPU",
        SortMode::Memory => "MEM",
    };
    let title = format!(" PROCESSES  {}  /  SORT {sort} ", processes.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(theme::OLIVE)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(theme::PANEL))
        .border_style(Style::default().fg(if app.focus == Panel::Processes {
            theme::OLIVE
        } else {
            theme::BORDER
        }));
    if area.height < 3 || area.width < 12 {
        frame.render_widget(block, area);
        return;
    }
    let wide = area.width >= 70;
    let header = if wide {
        Row::new(["PROCESS", "PID", "CPU", "MEMORY", "STATUS"])
    } else {
        Row::new(["PROCESS", "PID", "CPU", "MEMORY"])
    }
    .style(
        Style::default()
            .fg(theme::MUTED)
            .add_modifier(Modifier::BOLD),
    );
    let rows: Vec<Row> = if processes.is_empty() {
        vec![Row::new(if wide {
            vec!["No matching processes", "", "", "", ""]
        } else {
            vec!["No matching processes", "", "", ""]
        })]
    } else {
        processes
            .iter()
            .map(|process| process_row(process, wide))
            .collect()
    };
    let widths = if wide {
        vec![
            Constraint::Min(14),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Length(10),
            Constraint::Length(13),
        ]
    } else {
        vec![
            Constraint::Min(8),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(8),
        ]
    };
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(theme::BACKGROUND)
                .bg(theme::OLIVE)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ")
        .column_spacing(1);
    let mut state = TableState::default();
    if !processes.is_empty() {
        state.select(Some(app.selected_index));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn process_row(process: &ProcessInfo, wide: bool) -> Row<'static> {
    let mut cells = vec![
        process.name.clone(),
        process.identity.pid.to_string(),
        process
            .cpu_percent
            .map(|value| format!("{value:.1}%"))
            .unwrap_or_else(|| "N/A".into()),
        process
            .memory_bytes
            .map(bytes)
            .unwrap_or_else(|| "N/A".into()),
    ];
    if wide {
        cells.push(process.status.clone().unwrap_or_else(|| "N/A".into()));
    }
    Row::new(cells).style(Style::default().fg(theme::TEXT))
}

pub fn details(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let selected = app.selected_process();
    let warning = app
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.warnings.first());
    let lines = if let Some(process) = selected {
        vec![
            Line::from(vec![
                Span::styled(" PROCESS  ", Style::default().fg(theme::MUTED)),
                Span::styled(
                    process.name,
                    Style::default()
                        .fg(theme::TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  PID {}", process.identity.pid),
                    Style::default().fg(theme::OLIVE),
                ),
            ]),
            Line::from(vec![
                Span::styled(" COMMAND  ", Style::default().fg(theme::MUTED)),
                Span::styled(process.command, Style::default().fg(theme::TEXT)),
            ]),
            Line::from(vec![
                Span::styled(" USER  ", Style::default().fg(theme::MUTED)),
                Span::raw(process.user.unwrap_or_else(|| "N/A".into())),
                Span::styled("   STATE  ", Style::default().fg(theme::MUTED)),
                Span::raw(process.status.unwrap_or_else(|| "N/A".into())),
            ]),
        ]
    } else {
        vec![Line::from(" No process selected")]
    };
    let mut lines = lines;
    if let Some(notice) = &app.notice {
        lines.push(Line::from(Span::styled(
            format!(" EVENT  {notice}"),
            Style::default().fg(theme::YELLOW),
        )));
    } else if let Some(warning) = warning {
        lines.push(Line::from(Span::styled(
            format!(" WARNING  {}: {}", warning.collector, warning.message),
            Style::default().fg(theme::YELLOW),
        )));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" DETAILS / EVENTS ")
        .title_style(Style::default().fg(theme::OLIVE))
        .border_style(Style::default().fg(if app.focus == Panel::Details {
            theme::OLIVE
        } else {
            theme::BORDER
        }))
        .style(Style::default().bg(theme::PANEL).fg(theme::TEXT));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

pub fn footer(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let lines = if app.mode == AppMode::Filtering {
        vec![Line::from(vec![
            Span::styled(
                " FILTER  ",
                Style::default().fg(theme::BACKGROUND).bg(theme::OLIVE),
            ),
            Span::styled(
                format!(" {}_", app.filter),
                Style::default().fg(theme::TEXT),
            ),
            Span::styled(
                "   ENTER DONE  ESC CLOSE",
                Style::default().fg(theme::MUTED),
            ),
        ])]
    } else if area.width < 42 {
        vec![Line::from(vec![
            Span::styled("  Q ", Style::default().fg(theme::OLIVE)),
            Span::styled("QUIT   ", Style::default().fg(theme::MUTED)),
            Span::styled("F ", Style::default().fg(theme::OLIVE)),
            Span::styled("FILTER   ", Style::default().fg(theme::MUTED)),
            Span::styled("K ", Style::default().fg(theme::YELLOW)),
            Span::styled("ACTION", Style::default().fg(theme::MUTED)),
        ])]
    } else if area.width < 100 {
        vec![
            Line::from(vec![
                Span::styled("  ↑↓ ", Style::default().fg(theme::OLIVE)),
                Span::styled("SELECT  ", Style::default().fg(theme::MUTED)),
                Span::styled("ENTER ", Style::default().fg(theme::OLIVE)),
                Span::styled("DETAIL  ", Style::default().fg(theme::MUTED)),
                Span::styled("F ", Style::default().fg(theme::OLIVE)),
                Span::styled("FILTER  ", Style::default().fg(theme::MUTED)),
                Span::styled("Q ", Style::default().fg(theme::OLIVE)),
                Span::styled("QUIT", Style::default().fg(theme::MUTED)),
            ]),
            Line::from(vec![
                Span::styled("  TAB ", Style::default().fg(theme::OLIVE)),
                Span::styled("PANEL  ", Style::default().fg(theme::MUTED)),
                Span::styled("C/M ", Style::default().fg(theme::OLIVE)),
                Span::styled("SORT  ", Style::default().fg(theme::MUTED)),
                Span::styled("K/⇧K ", Style::default().fg(theme::YELLOW)),
                Span::styled("ACTION  ", Style::default().fg(theme::MUTED)),
                Span::styled("R ", Style::default().fg(theme::OLIVE)),
                Span::styled("REFRESH", Style::default().fg(theme::MUTED)),
            ]),
        ]
    } else {
        vec![Line::from(vec![
            Span::styled("  TAB ", Style::default().fg(theme::OLIVE)),
            Span::styled("PANEL   ", Style::default().fg(theme::MUTED)),
            Span::styled("↑↓ ", Style::default().fg(theme::OLIVE)),
            Span::styled("SELECT   ", Style::default().fg(theme::MUTED)),
            Span::styled("ENTER ", Style::default().fg(theme::OLIVE)),
            Span::styled("DETAIL   ", Style::default().fg(theme::MUTED)),
            Span::styled("F ", Style::default().fg(theme::OLIVE)),
            Span::styled("FILTER   ", Style::default().fg(theme::MUTED)),
            Span::styled("C/M ", Style::default().fg(theme::OLIVE)),
            Span::styled("SORT   ", Style::default().fg(theme::MUTED)),
            Span::styled("K/⇧K ", Style::default().fg(theme::YELLOW)),
            Span::styled("ACTION   ", Style::default().fg(theme::MUTED)),
            Span::styled("R ", Style::default().fg(theme::OLIVE)),
            Span::styled("REFRESH   ", Style::default().fg(theme::MUTED)),
            Span::styled("Q ", Style::default().fg(theme::OLIVE)),
            Span::styled("QUIT", Style::default().fg(theme::MUTED)),
        ])]
    };
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme::PANEL)),
        area,
    );
}

pub fn detail_modal(frame: &mut Frame, area: Rect, app: &App) {
    let Some(process) = app.selected_process() else {
        return;
    };
    let popup = centered(area, 70, 11);
    if popup.width < 4 || popup.height < 4 {
        return;
    }
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
            Style::default().fg(theme::OLIVE),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" PROCESS DETAIL ")
        .border_style(Style::default().fg(theme::OLIVE))
        .style(Style::default().fg(theme::TEXT).bg(theme::PANEL));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        popup,
    );
}

pub fn confirmation_modal(frame: &mut Frame, area: Rect, app: &App) {
    let Some(pending) = app.pending_action else {
        return;
    };
    let popup = centered(area, 56, 9);
    if popup.width < 4 || popup.height < 4 {
        return;
    }
    let name = app
        .snapshot
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .processes
                .iter()
                .find(|process| process.identity == pending.identity())
        })
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
            Style::default().fg(theme::RED).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("{name}  /  PID {}", pending.identity().pid)),
        Line::from(""),
        Line::from(Span::styled(
            "Y  CONFIRM       N / ESC  CANCEL",
            Style::default().fg(theme::YELLOW),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" CONFIRM ACTION ")
        .title_style(Style::default().fg(theme::RED).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(theme::RED))
        .style(Style::default().fg(theme::TEXT).bg(theme::PANEL));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center),
        popup,
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

fn value(snapshot: &SystemSnapshot, name: &str) -> Option<f64> {
    snapshot
        .metrics
        .iter()
        .find(|metric| metric.name == name)?
        .value
}

fn disk_usage(snapshot: &SystemSnapshot) -> Option<f64> {
    value(snapshot, "disk./.usage").or_else(|| {
        snapshot
            .metrics
            .iter()
            .find(|metric| metric.name.starts_with("disk.") && metric.name.ends_with(".usage"))
            .and_then(|metric| metric.value)
    })
}

fn disk_capacity(snapshot: Option<&SystemSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "N/A".into();
    };
    let metric = snapshot
        .metrics
        .iter()
        .find(|metric| metric.name == "disk./.total")
        .or_else(|| {
            snapshot
                .metrics
                .iter()
                .find(|metric| metric.name.starts_with("disk.") && metric.name.ends_with(".total"))
        });
    metric
        .and_then(|metric| metric.value)
        .map(|total| format!("{} TOTAL", bytes(total as u64)))
        .unwrap_or_else(|| "N/A".into())
}

fn memory_capacity(snapshot: Option<&SystemSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "N/A".into();
    };
    match (
        value(snapshot, "memory.used"),
        value(snapshot, "memory.total"),
    ) {
        (Some(used), Some(total)) => format!("{} / {}", bytes(used as u64), bytes(total as u64)),
        _ => "N/A".into(),
    }
}

fn network_rates(snapshot: &SystemSnapshot) -> (Option<f64>, Option<f64>) {
    let mut rx = Vec::new();
    let mut tx = Vec::new();
    for metric in &snapshot.metrics {
        if metric.name.starts_with("network.") {
            if metric.name.ends_with(".received") {
                rx.extend(metric.value);
            }
            if metric.name.ends_with(".transmitted") {
                tx.extend(metric.value);
            }
        }
    }
    (
        (!rx.is_empty()).then(|| rx.iter().sum()),
        (!tx.is_empty()).then(|| tx.iter().sum()),
    )
}

fn percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0}%"))
        .unwrap_or_else(|| "N/A".into())
}

fn bytes(value: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = value as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1}{}", UNITS[unit])
}

fn rate(value: f64) -> String {
    format!("{}/s", bytes(value.max(0.0) as u64))
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = seconds % 86_400 / 3_600;
    let minutes = seconds % 3_600 / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else {
        format!("{hours}h {minutes}m")
    }
}

fn usage_color(value: f64) -> ratatui::style::Color {
    if value >= 85.0 {
        theme::RED
    } else if value >= 70.0 {
        theme::YELLOW
    } else {
        theme::GREEN
    }
}
