use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Row, Sparkline, Table, TableState, Wrap},
    Frame,
};

use crate::app::{App, Panel};
use crate::model::{ActionKind, AppMode, NetworkSummary, ProcessInfo, SortMode, SystemSnapshot};

use super::theme;

pub fn status(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.ui.theme.palette();
    let snapshot = app.snapshot.as_ref();
    let warnings = snapshot.map_or(0, |snapshot| snapshot.warnings.len());
    let health = if warnings == 0 {
        "SYSTEM NORMAL"
    } else {
        "CHECK METRICS"
    };
    let health_color = if warnings == 0 {
        palette.green
    } else {
        palette.yellow
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
                    .fg(palette.background)
                    .bg(palette.olive)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  SYSTEM MONITOR  /  ", Style::default().fg(palette.muted)),
            Span::styled(
                health,
                Style::default()
                    .fg(health_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  /  {}", app.ui.workspace.label()),
                Style::default().fg(palette.olive),
            ),
        ]),
        Line::from(vec![
            Span::styled("  UPTIME ", Style::default().fg(palette.muted)),
            Span::styled(uptime, Style::default().fg(palette.text)),
            Span::styled("   BATTERY ", Style::default().fg(palette.muted)),
            Span::styled(battery, Style::default().fg(palette.text)),
            Span::styled("   TEMP ", Style::default().fg(palette.muted)),
            Span::styled(temperature, Style::default().fg(palette.text)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.panel)),
        area,
    );
}

pub fn resources(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let snapshot = app.snapshot.as_ref();
    let cpu = snapshot.and_then(|s| value(s, "cpu.usage"));
    let memory = snapshot.and_then(|s| value(s, "memory.usage"));
    let disk = snapshot.and_then(disk_usage);
    let (rx, tx) = snapshot.map(network_rates).unwrap_or((None, None));
    let cards = [
        (
            "CPU",
            percent(cpu),
            cpu_detail(snapshot),
            cpu,
            &app.histories[0],
        ),
        (
            "MEMORY",
            percent(memory),
            memory_detail(snapshot),
            memory,
            &app.histories[1],
        ),
        (
            "DISK",
            percent(disk),
            disk_detail(snapshot),
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
            resource_card(
                frame,
                columns[column],
                card,
                app.focus == Panel::Resources,
                &palette,
            );
        }
    }
}

fn resource_card(
    frame: &mut Frame,
    area: Rect,
    card: &(&str, String, String, Option<f64>, &Vec<u64>),
    focused: bool,
    palette: &theme::Palette,
) {
    let (title, main, secondary, usage, history) = card;
    if area.width < 3 || area.height < 2 {
        return;
    }
    let border = if focused {
        palette.olive
    } else {
        palette.border
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(palette.muted)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(palette.panel).fg(palette.text))
        .border_style(Style::default().fg(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let color = usage
        .map(|value| usage_color(value, palette))
        .unwrap_or(palette.olive);
    frame.render_widget(
        Paragraph::new(main.to_owned()).style(
            Style::default()
                .fg(color)
                .bg(palette.panel)
                .add_modifier(Modifier::BOLD),
        ),
        Rect { height: 1, ..inner },
    );
    if inner.height > 1 {
        frame.render_widget(
            Paragraph::new(secondary.to_owned())
                .style(Style::default().fg(palette.muted).bg(palette.panel)),
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
                .style(Style::default().fg(color).bg(palette.panel)),
            spark_area,
        );
    }
}

pub fn processes(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
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
                .fg(palette.olive)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(palette.panel))
        .border_style(Style::default().fg(if app.focus == Panel::Processes {
            palette.olive
        } else {
            palette.border
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
            .fg(palette.muted)
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
                .fg(palette.background)
                .bg(palette.olive)
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

pub fn network(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let summaries = app
        .snapshot
        .as_ref()
        .map(|snapshot| snapshot.network_summaries.as_slice())
        .unwrap_or(&[]);
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
    let (download, upload) = summaries.iter().fold((0.0, 0.0), |(down, up), item| {
        (
            down + item.download_rate.unwrap_or(0.0),
            up + item.upload_rate.unwrap_or(0.0),
        )
    });
    network_card(
        frame,
        columns[0],
        "DOWNLOAD",
        rate_or_na(if summaries.is_empty() {
            None
        } else {
            Some(download)
        }),
        palette.green,
        &palette,
    );
    network_card(
        frame,
        columns[1],
        "UPLOAD",
        rate_or_na(if summaries.is_empty() {
            None
        } else {
            Some(upload)
        }),
        palette.olive,
        &palette,
    );
    let body = Rect {
        y: area.y.saturating_add(4),
        height: area.height.saturating_sub(4),
        ..area
    };
    let rows = summaries.iter().map(network_row).collect::<Vec<_>>();
    let table = Table::new(
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
                .fg(palette.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" NETWORK / DOWNLOAD ")
            .title_style(Style::default().fg(palette.olive))
            .border_style(Style::default().fg(palette.border))
            .style(Style::default().bg(palette.panel)),
    )
    .style(Style::default().fg(palette.text))
    .column_spacing(1);
    frame.render_widget(table, body);
}

pub fn disks(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let rows = app
        .snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot
                .metrics
                .iter()
                .filter(|metric| {
                    metric.name.starts_with("disk.") && metric.name.ends_with(".usage")
                })
                .map(|metric| {
                    let mount = metric
                        .name
                        .strip_prefix("disk.")
                        .unwrap_or("N/A")
                        .strip_suffix(".usage")
                        .unwrap_or("N/A");
                    let prefix = format!("disk.{mount}");
                    let find = |suffix: &str| {
                        snapshot
                            .metrics
                            .iter()
                            .find(|item| item.name == format!("{prefix}.{suffix}"))
                            .and_then(|item| item.value)
                    };
                    Row::new([
                        mount.to_owned(),
                        metric
                            .value
                            .map(|value| format!("{value:.0}%"))
                            .unwrap_or_else(|| "N/A".into()),
                        find("used")
                            .map(|value| bytes(value as u64))
                            .unwrap_or_else(|| "N/A".into()),
                        find("free")
                            .map(|value| bytes(value as u64))
                            .unwrap_or_else(|| "N/A".into()),
                        find("total")
                            .map(|value| bytes(value as u64))
                            .unwrap_or_else(|| "N/A".into()),
                    ])
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
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
        Row::new(["MOUNT", "USED", "USED", "FREE", "TOTAL"]).style(
            Style::default()
                .fg(palette.muted)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" DISKS ")
            .title_style(Style::default().fg(palette.olive))
            .border_style(Style::default().fg(palette.border))
            .style(Style::default().bg(palette.panel)),
    )
    .style(Style::default().fg(palette.text));
    frame.render_widget(table, area);
}

pub fn more(frame: &mut Frame, area: Rect, app: &App) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let palette = app.ui.theme.palette();
    let lines = vec![
        Line::from(Span::styled(
            "  FILIZ CONTROL CENTER",
            Style::default()
                .fg(palette.olive)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("  THEME    {}", app.ui.theme.label())),
        Line::from(format!("  DENSITY  {:?}", app.ui.density)),
        Line::from("  MENU     M  toggle layout options"),
        Line::from(""),
        Line::from(Span::styled(
            "  for betül, with love ♡",
            Style::default().fg(palette.yellow),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" MORE / SETTINGS ")
        .title_style(Style::default().fg(palette.olive))
        .border_style(Style::default().fg(palette.border))
        .style(Style::default().bg(palette.panel));
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
    palette: &theme::Palette,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(palette.muted)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(palette.border))
        .style(Style::default().bg(palette.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(main).style(
            Style::default()
                .fg(color)
                .bg(palette.panel)
                .add_modifier(Modifier::BOLD),
        ),
        inner,
    );
}

fn network_row(summary: &NetworkSummary) -> Row<'static> {
    let total = summary.download_total.saturating_add(summary.upload_total);
    Row::new([
        summary.interface.clone(),
        summary
            .download_rate
            .map(rate)
            .unwrap_or_else(|| "N/A".into()),
        summary
            .upload_rate
            .map(rate)
            .unwrap_or_else(|| "N/A".into()),
        bytes(total),
        format!(
            "↓ {} ↑ {}",
            rate(summary.peak_download),
            rate(summary.peak_upload)
        ),
    ])
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
    let warning = app
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.warnings.first());
    let mut lines = Vec::new();
    if let Some(notice) = &app.notice {
        lines.push(Line::from(Span::styled(
            format!(" EVENT  {notice}"),
            Style::default()
                .fg(palette.yellow)
                .add_modifier(Modifier::BOLD),
        )));
    } else if let Some(warning) = warning {
        lines.push(Line::from(Span::styled(
            format!(" WARNING  {}: {}", warning.collector, warning.message),
            Style::default()
                .fg(palette.yellow)
                .add_modifier(Modifier::BOLD),
        )));
    }
    if let Some(process) = selected {
        lines.extend([
            Line::from(vec![
                Span::styled(" PROCESS  ", Style::default().fg(palette.muted)),
                Span::styled(
                    process.name,
                    Style::default()
                        .fg(palette.text)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  PID {}", process.identity.pid),
                    Style::default().fg(palette.olive),
                ),
            ]),
            Line::from(vec![
                Span::styled(" COMMAND  ", Style::default().fg(palette.muted)),
                Span::styled(process.command, Style::default().fg(palette.text)),
            ]),
            Line::from(vec![
                Span::styled(" USER  ", Style::default().fg(palette.muted)),
                Span::raw(process.user.unwrap_or_else(|| "N/A".into())),
                Span::styled("   STATE  ", Style::default().fg(palette.muted)),
                Span::raw(process.status.unwrap_or_else(|| "N/A".into())),
            ]),
        ]);
    } else {
        lines.push(Line::from(" No process selected"));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" DETAILS / EVENTS ")
        .title_style(Style::default().fg(palette.olive))
        .border_style(Style::default().fg(if app.focus == Panel::Details {
            palette.olive
        } else {
            palette.border
        }))
        .style(Style::default().bg(palette.panel).fg(palette.text));
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
    let lines = if app.mode == AppMode::Filtering {
        vec![Line::from(vec![
            Span::styled(
                " FILTER  ",
                Style::default().fg(palette.background).bg(palette.olive),
            ),
            Span::styled(
                format!(" {}_", app.filter),
                Style::default().fg(palette.text),
            ),
            Span::styled(
                "   ENTER DONE  ESC CLOSE",
                Style::default().fg(palette.muted),
            ),
        ])]
    } else if area.width < 42 {
        vec![Line::from(vec![
            Span::styled("  Q ", Style::default().fg(palette.olive)),
            Span::styled("QUIT   ", Style::default().fg(palette.muted)),
            Span::styled("F ", Style::default().fg(palette.olive)),
            Span::styled("FILTER   ", Style::default().fg(palette.muted)),
            Span::styled("K ", Style::default().fg(palette.yellow)),
            Span::styled("ACTION", Style::default().fg(palette.muted)),
        ])]
    } else if area.width < 100 {
        vec![
            Line::from(vec![
                Span::styled("  ↑↓ ", Style::default().fg(palette.olive)),
                Span::styled("SELECT  ", Style::default().fg(palette.muted)),
                Span::styled("ENTER ", Style::default().fg(palette.olive)),
                Span::styled("DETAIL  ", Style::default().fg(palette.muted)),
                Span::styled("F ", Style::default().fg(palette.olive)),
                Span::styled("FILTER  ", Style::default().fg(palette.muted)),
                Span::styled("Q ", Style::default().fg(palette.olive)),
                Span::styled("QUIT", Style::default().fg(palette.muted)),
            ]),
            Line::from(vec![
                Span::styled("  TAB ", Style::default().fg(palette.olive)),
                Span::styled("PANEL  ", Style::default().fg(palette.muted)),
                Span::styled("C/M ", Style::default().fg(palette.olive)),
                Span::styled("SORT  ", Style::default().fg(palette.muted)),
                Span::styled("K/⇧K ", Style::default().fg(palette.yellow)),
                Span::styled("ACTION  ", Style::default().fg(palette.muted)),
                Span::styled("R ", Style::default().fg(palette.olive)),
                Span::styled("REFRESH", Style::default().fg(palette.muted)),
            ]),
        ]
    } else {
        vec![Line::from(vec![
            Span::styled("  TAB ", Style::default().fg(palette.olive)),
            Span::styled("PANEL   ", Style::default().fg(palette.muted)),
            Span::styled("↑↓ ", Style::default().fg(palette.olive)),
            Span::styled("SELECT   ", Style::default().fg(palette.muted)),
            Span::styled("ENTER ", Style::default().fg(palette.olive)),
            Span::styled("DETAIL   ", Style::default().fg(palette.muted)),
            Span::styled("F ", Style::default().fg(palette.olive)),
            Span::styled("FILTER   ", Style::default().fg(palette.muted)),
            Span::styled("C/M ", Style::default().fg(palette.olive)),
            Span::styled("SORT   ", Style::default().fg(palette.muted)),
            Span::styled("K/⇧K ", Style::default().fg(palette.yellow)),
            Span::styled("ACTION   ", Style::default().fg(palette.muted)),
            Span::styled("R ", Style::default().fg(palette.olive)),
            Span::styled("REFRESH   ", Style::default().fg(palette.muted)),
            Span::styled("Q ", Style::default().fg(palette.olive)),
            Span::styled("QUIT", Style::default().fg(palette.muted)),
        ])]
    };
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette.panel)),
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
            Style::default().fg(palette.olive),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" PROCESS DETAIL ")
        .border_style(Style::default().fg(palette.olive))
        .style(Style::default().fg(palette.text).bg(palette.panel));
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
            Style::default()
                .fg(palette.red)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("{name}  /  PID {}", pending.identity().pid)),
        Line::from(""),
        Line::from(Span::styled(
            "Y  CONFIRM       N / ESC  CANCEL",
            Style::default().fg(palette.yellow),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" CONFIRM ACTION ")
        .title_style(
            Style::default()
                .fg(palette.red)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(palette.red))
        .style(Style::default().fg(palette.text).bg(palette.panel));
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

fn disk_metric(snapshot: &SystemSnapshot, suffix: &str) -> Option<f64> {
    value(snapshot, &format!("disk./.{suffix}")).or_else(|| {
        snapshot
            .metrics
            .iter()
            .find(|metric| {
                metric.name.starts_with("disk.") && metric.name.ends_with(&format!(".{suffix}"))
            })
            .and_then(|metric| metric.value)
    })
}

fn cpu_detail(snapshot: Option<&SystemSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "CORES N/A · IDLE N/A".into();
    };
    let cores = value(snapshot, "cpu.cores")
        .map(|cores| format!("{cores:.0} CORES"))
        .unwrap_or_else(|| "CORES N/A".into());
    let idle = value(snapshot, "cpu.idle")
        .map(|idle| format!("IDLE {idle:.0}%"))
        .unwrap_or_else(|| "IDLE N/A".into());
    format!("{cores} · {idle}")
}

fn memory_detail(snapshot: Option<&SystemSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "USED N/A · FREE N/A".into();
    };
    let used = value(snapshot, "memory.used").map(|value| bytes(value as u64));
    let free = value(snapshot, "memory.free").map(|value| bytes(value as u64));
    let available = value(snapshot, "memory.available").map(|value| bytes(value as u64));
    format!(
        "USED {} · FREE {} · AVAIL {}",
        used.unwrap_or_else(|| "N/A".into()),
        free.unwrap_or_else(|| "N/A".into()),
        available.unwrap_or_else(|| "N/A".into())
    )
}

fn disk_detail(snapshot: Option<&SystemSnapshot>) -> String {
    let Some(snapshot) = snapshot else {
        return "USED N/A · FREE N/A".into();
    };
    format!(
        "USED {} · FREE {} · TOTAL {}",
        disk_metric(snapshot, "used")
            .map(|value| bytes(value as u64))
            .unwrap_or_else(|| "N/A".into()),
        disk_metric(snapshot, "free")
            .map(|value| bytes(value as u64))
            .unwrap_or_else(|| "N/A".into()),
        disk_metric(snapshot, "total")
            .map(|value| bytes(value as u64))
            .unwrap_or_else(|| "N/A".into())
    )
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

fn usage_color(value: f64, palette: &theme::Palette) -> ratatui::style::Color {
    if value >= 85.0 {
        palette.red
    } else if value >= 70.0 {
        palette.yellow
    } else {
        palette.green
    }
}
