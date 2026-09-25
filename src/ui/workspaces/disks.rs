use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::state::DiskStats;
use crate::ui::components::table::{border_for, highlight, register_rows, table_state};
use crate::ui::format::{bytes, percent};
use crate::ui::state::PanelId;
use crate::ui::RenderCx;

use super::WorkspaceView;

pub struct Disks;

impl WorkspaceView for Disks {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        disks(frame, area, cx);
    }
}

fn disks(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    cx.out
        .viewports
        .insert(PanelId::Disks, area.height.saturating_sub(3) as usize);
    if area.height == 0 || area.width == 0 {
        return;
    }
    let app = cx.app;
    let palette = cx.palette;
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
            .border_style(border_for(cx, PanelId::Disks))
            .style(Style::default().bg(palette.surface)),
    )
    .style(Style::default().fg(palette.text))
    .row_highlight_style(highlight(&palette))
    .highlight_symbol("▸ ");
    let len = disks.len();
    frame.render_stateful_widget(table, area, &mut table_state(app, PanelId::Disks, len));
    register_rows(cx, PanelId::Disks, area, len);
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
