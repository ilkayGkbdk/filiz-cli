use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::model::{ProcessInfo, SortMode};
use crate::ui::components::table::{border_for, highlight, register_rows, table_state};
use crate::ui::format::bytes;
use crate::ui::state::{LayoutDensity, PanelId};
use crate::ui::theme;
use crate::ui::RenderCx;

use super::WorkspaceView;

pub struct Processes;

impl WorkspaceView for Processes {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        let detail = match cx.app.ui.density {
            LayoutDensity::Compact => 3,
            LayoutDensity::Balanced => 6,
            LayoutDensity::Spacious => 8,
        };
        let constraints = match (cx.visible(PanelId::Processes), cx.visible(PanelId::Details)) {
            (true, true) => [Constraint::Min(0), Constraint::Length(detail)],
            (true, false) => [Constraint::Min(0), Constraint::Length(0)],
            _ => [Constraint::Length(0), Constraint::Min(0)],
        };
        let [table, detail_area] = Layout::vertical(constraints).areas(area);
        if cx.visible(PanelId::Processes) {
            process_table(frame, table, cx);
        }
        if cx.visible(PanelId::Details) {
            super::overview::details(frame, detail_area, cx);
        }
    }
}

pub(crate) fn process_table(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    cx.out
        .viewports
        .insert(PanelId::Processes, area.height.saturating_sub(3) as usize);
    if area.height == 0 || area.width == 0 {
        return;
    }
    let app = cx.app;
    let palette = cx.palette;
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
        .border_style(border_for(cx, PanelId::Processes));
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
        .row_highlight_style(highlight(&palette))
        .highlight_symbol("▸ ")
        .column_spacing(1);
    let mut state = table_state(app, PanelId::Processes, processes.len());
    let len = processes.len();
    frame.render_stateful_widget(table, area, &mut state);
    register_rows(cx, PanelId::Processes, area, len);
}

pub(crate) fn process_row(
    process: &ProcessInfo,
    wide: bool,
    palette: &theme::Palette,
) -> Row<'static> {
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
