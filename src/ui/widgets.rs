use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Sparkline, Table, TableState, Wrap},
    Frame,
};

use crate::app::App;
use crate::history::SeriesKey;
use crate::input::action::Action;
use crate::input::keymap;
use crate::model::{ActionKind, AppMode, ProcessInfo, SortMode};
use crate::state::{DiskStats, InterfaceStats, SystemState};

use super::format::{bytes, percent, rate, uptime as format_uptime};
use super::state::{PanelId, Workspace};
use super::theme;
use super::RenderOutput;

fn table_state(app: &App, panel: PanelId, len: usize) -> TableState {
    let list = app.ui.list(panel);
    let state = TableState::default().with_offset(list.offset);
    if len == 0 {
        state
    } else {
        state.with_selected(Some(list.selected))
    }
}

fn border_for(app: &App, panel: PanelId, palette: &theme::Palette) -> Style {
    Style::default().fg(if app.ui.focus == panel {
        palette.border_focus
    } else {
        palette.border
    })
}

pub fn status(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.ui.theme.palette();
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
        let tabs = Workspace::ALL
            .iter()
            .enumerate()
            .map(|(index, workspace)| {
                let style = if *workspace == app.ui.workspace {
                    Style::default()
                        .fg(palette.bg)
                        .bg(palette.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette.text_muted)
                };
                Span::styled(format!("  {} {}  ", index + 1, workspace.label()), style)
            })
            .collect::<Vec<_>>();
        lines.push(Line::from(tabs));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.surface)),
        area,
    );
}

pub fn resources(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
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
                app.ui.focus == PanelId::Resources,
                &palette,
            );
        }
    }
}

fn resource_card(
    frame: &mut Frame,
    area: Rect,
    card: &(&str, String, String, Option<f64>, Vec<u64>),
    focused: bool,
    palette: &theme::Palette,
) {
    let (title, main, secondary, usage, history) = card;
    if area.width < 3 || area.height < 2 {
        return;
    }
    let border = if focused {
        palette.accent
    } else {
        palette.border
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(palette.surface).fg(palette.text))
        .border_style(Style::default().fg(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let color = usage
        .map(|value| usage_color(value, palette))
        .unwrap_or(palette.accent);
    frame.render_widget(
        Paragraph::new(main.to_owned()).style(
            Style::default()
                .fg(color)
                .bg(palette.surface)
                .add_modifier(Modifier::BOLD),
        ),
        Rect { height: 1, ..inner },
    );
    if inner.height > 1 {
        frame.render_widget(
            Paragraph::new(secondary.to_owned())
                .style(Style::default().fg(palette.text_muted).bg(palette.surface)),
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
                .data(history)
                .max(if usage.is_some() {
                    100
                } else {
                    history.iter().copied().max().unwrap_or(1).max(1)
                })
                .style(Style::default().fg(color).bg(palette.surface)),
            spark_area,
        );
    }
}

pub fn processes(frame: &mut Frame, area: Rect, app: &App, out: &mut RenderOutput) {
    out.viewports
        .insert(PanelId::Processes, area.height.saturating_sub(3) as usize);
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let processes = app.visible_processes();
    let title = format!(" PROCESSES  {} ", processes.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(palette.surface))
        .border_style(border_for(app, PanelId::Processes, &palette));
    if area.height < 3 || area.width < 12 {
        frame.render_widget(block, area);
        return;
    }
    let wide = area.width >= 70;
    let cpu_label = if app.sort == SortMode::Cpu {
        "CPU ▼"
    } else {
        "CPU"
    };
    let memory_label = if app.sort == SortMode::Memory {
        "MEMORY ▼"
    } else {
        "MEMORY"
    };
    let header_cells: Vec<&str> = if wide {
        vec!["PROCESS", "PID", cpu_label, memory_label, "STATUS"]
    } else {
        vec!["PROCESS", "PID", cpu_label, memory_label]
    };
    let header = Row::new(header_cells).style(
        Style::default()
            .fg(palette.text_muted)
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
            .map(|process| process_row(process, wide, &palette))
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
                .fg(palette.selection_fg)
                .bg(palette.selection_bg),
        )
        .highlight_symbol("▸ ")
        .column_spacing(1);
    let mut state = table_state(app, PanelId::Processes, processes.len());
    frame.render_stateful_widget(table, area, &mut state);
}

pub fn network(frame: &mut Frame, area: Rect, app: &App, out: &mut RenderOutput) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let selected = app.selected_interface();
    let sections = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Min(0),
    ])
    .split(area);
    let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[0]);
    let download_history = selected
        .map(|i| app.history.series(&SeriesKey::NetRx(i.name.clone())))
        .unwrap_or_default();
    let upload_history = selected
        .map(|i| app.history.series(&SeriesKey::NetTx(i.name.clone())))
        .unwrap_or_default();
    network_card(
        frame,
        columns[0],
        "DOWNLOAD",
        rate_or_na(selected.and_then(|i| i.rx_rate)),
        palette.chart_rx,
        &download_history,
        &palette,
    );
    network_card(
        frame,
        columns[1],
        "UPLOAD",
        rate_or_na(selected.and_then(|i| i.tx_rate)),
        palette.chart_tx,
        &upload_history,
        &palette,
    );
    let rows = app
        .state
        .interfaces
        .iter()
        .map(network_row)
        .collect::<Vec<_>>();
    let interfaces_table = Table::new(
        if rows.is_empty() {
            vec![Row::new(["No network interfaces", "", "", "", ""])]
        } else {
            rows
        },
        [
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Min(12),
        ],
    )
    .header(
        Row::new(["INTERFACE", "DOWN", "UP", "TOTAL", "PEAK"]).style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " NETWORK / DOWNLOAD  {} ",
                selected.map(|item| item.name.as_str()).unwrap_or("N/A")
            ))
            .title_style(Style::default().fg(palette.accent))
            .border_style(border_for(app, PanelId::Interfaces, &palette))
            .style(Style::default().bg(palette.surface)),
    )
    .style(Style::default().fg(palette.text))
    .row_highlight_style(
        Style::default()
            .fg(palette.selection_fg)
            .bg(palette.selection_bg),
    )
    .highlight_symbol("▸ ")
    .column_spacing(1);
    frame.render_stateful_widget(
        interfaces_table,
        sections[1],
        &mut table_state(app, PanelId::Interfaces, app.state.interfaces.len()),
    );
    out.viewports.insert(
        PanelId::Interfaces,
        sections[1].height.saturating_sub(3) as usize,
    );

    let talkers = app.traffic_rows();
    let empty = if app.state.issue(crate::state::Source::Traffic).is_some() {
        "Traffic unavailable"
    } else {
        "No process traffic yet"
    };
    let rows: Vec<Row> = talkers.iter().map(|p| traffic_row(p)).collect();
    let traffic_table = Table::new(
        if rows.is_empty() {
            vec![Row::new([empty, "", "", ""])]
        } else {
            rows
        },
        [
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(["PROCESS", "PID", "DOWN", "UP"]).style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" PROCESS TRAFFIC ({}) ", talkers.len()))
            .title_style(Style::default().fg(palette.accent))
            .border_style(border_for(app, PanelId::Traffic, &palette))
            .style(Style::default().bg(palette.surface)),
    )
    .style(Style::default().fg(palette.text))
    .row_highlight_style(
        Style::default()
            .fg(palette.selection_fg)
            .bg(palette.selection_bg),
    )
    .highlight_symbol("▸ ");
    frame.render_stateful_widget(
        traffic_table,
        sections[2],
        &mut table_state(app, PanelId::Traffic, talkers.len()),
    );
    out.viewports.insert(
        PanelId::Traffic,
        sections[2].height.saturating_sub(3) as usize,
    );
}

pub fn disks(frame: &mut Frame, area: Rect, app: &App, out: &mut RenderOutput) {
    out.viewports
        .insert(PanelId::Disks, area.height.saturating_sub(3) as usize);
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let disks = app.state.visible_disks();
    let rows = disks.iter().map(|disk| disk_row(disk)).collect::<Vec<_>>();
    let table = Table::new(
        if rows.is_empty() {
            vec![Row::new(["No disks", "", "", "", ""])]
        } else {
            rows
        },
        [
            Constraint::Length(18),
            Constraint::Length(10),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(14),
        ],
    )
    .header(
        Row::new(["MOUNT", "USAGE", "USED", "FREE", "TOTAL"]).style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" DISKS ")
            .title_style(Style::default().fg(palette.accent))
            .border_style(border_for(app, PanelId::Disks, &palette))
            .style(Style::default().bg(palette.surface)),
    )
    .style(Style::default().fg(palette.text))
    .row_highlight_style(
        Style::default()
            .fg(palette.selection_fg)
            .bg(palette.selection_bg),
    )
    .highlight_symbol("▸ ");
    frame.render_stateful_widget(
        table,
        area,
        &mut table_state(app, PanelId::Disks, disks.len()),
    );
}

pub fn more(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
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

fn network_card(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    main: String,
    color: ratatui::style::Color,
    history: &[u64],
    palette: &theme::Palette,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(palette.border))
        .style(Style::default().bg(palette.surface));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(main).style(
            Style::default()
                .fg(color)
                .bg(palette.surface)
                .add_modifier(Modifier::BOLD),
        ),
        inner,
    );
    if inner.height > 1 && !history.is_empty() {
        frame.render_widget(
            Sparkline::default()
                .data(history)
                .max(history.iter().copied().max().unwrap_or(1).max(1))
                .style(Style::default().fg(color).bg(palette.surface)),
            Rect {
                y: inner.y + 1,
                height: inner.height - 1,
                ..inner
            },
        );
    }
}

fn rate_or_na(value: Option<f64>) -> String {
    value.map(rate).unwrap_or_else(|| "N/A".into())
}

fn process_row(process: &ProcessInfo, wide: bool, palette: &theme::Palette) -> Row<'static> {
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
    Row::new(cells).style(Style::default().fg(palette.text))
}

pub fn details(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
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
        .border_style(border_for(app, PanelId::Details, &palette))
        .style(Style::default().bg(palette.surface).fg(palette.text));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

pub fn footer(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
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

pub fn detail_modal(frame: &mut Frame, area: Rect, app: &App) {
    let Some(process) = app.selected_process() else {
        return;
    };
    let popup = centered(area, 70, 11);
    if popup.width < 4 || popup.height < 4 {
        return;
    }
    let palette = app.ui.theme.palette();
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

pub fn confirmation_modal(frame: &mut Frame, area: Rect, app: &App) {
    let Some(pending) = app.pending_action else {
        return;
    };
    let popup = centered(area, 56, 9);
    if popup.width < 4 || popup.height < 4 {
        return;
    }
    let palette = app.ui.theme.palette();
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
            "Y  CONFIRM       N / ESC  CANCEL",
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

fn network_row(interface: &InterfaceStats) -> Row<'static> {
    Row::new([
        interface.name.clone(),
        interface.rx_rate.map(rate).unwrap_or_else(|| "N/A".into()),
        interface.tx_rate.map(rate).unwrap_or_else(|| "N/A".into()),
        bytes(interface.rx_total.saturating_add(interface.tx_total)),
        format!(
            "↓ {} ↑ {}",
            rate(interface.peak_rx),
            rate(interface.peak_tx)
        ),
    ])
}

fn traffic_row(process: &ProcessInfo) -> Row<'static> {
    let traffic = process.traffic;
    Row::new([
        process.name.clone(),
        process.identity.pid.to_string(),
        traffic.map(|t| rate(t.rx)).unwrap_or_else(|| "N/A".into()),
        traffic.map(|t| rate(t.tx)).unwrap_or_else(|| "N/A".into()),
    ])
}

fn disk_row(disk: &DiskStats) -> Row<'static> {
    Row::new([
        disk.mount.clone(),
        percent(disk.usage_percent()),
        bytes(disk.used),
        bytes(disk.free),
        bytes(disk.total),
    ])
}

fn usage_color(value: f64, palette: &theme::Palette) -> ratatui::style::Color {
    if value >= 85.0 {
        palette.danger
    } else if value >= 70.0 {
        palette.warn
    } else {
        palette.ok
    }
}
