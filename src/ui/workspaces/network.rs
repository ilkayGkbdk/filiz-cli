use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::history::SeriesKey;
use crate::model::ProcessInfo;
use crate::state::InterfaceStats;
use crate::ui::components::card::network_card;
use crate::ui::components::table::{border_for, highlight, table_state};
use crate::ui::format::{bytes, rate};
use crate::ui::state::{LayoutDensity, PanelId};
use crate::ui::RenderCx;

use super::WorkspaceView;

pub struct Network;

impl WorkspaceView for Network {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let cards = match cx.app.ui.density {
            LayoutDensity::Compact => 4,
            LayoutDensity::Balanced => 6,
            LayoutDensity::Spacious => 8,
        };
        let interfaces = (cx.app.state.interfaces.len() as u16 + 3).min(8);
        let (interface_constraint, traffic_constraint) = match (
            cx.visible(PanelId::Interfaces),
            cx.visible(PanelId::Traffic),
        ) {
            (true, true) => (Constraint::Length(interfaces), Constraint::Min(0)),
            (true, false) => (Constraint::Min(0), Constraint::Length(0)),
            _ => (Constraint::Length(0), Constraint::Min(0)),
        };
        let [card_area, interface_area, traffic_area] = Layout::vertical([
            Constraint::Length(cards),
            interface_constraint,
            traffic_constraint,
        ])
        .areas(area);
        rate_cards(frame, card_area, cx);
        if cx.visible(PanelId::Interfaces) {
            interface_table(frame, interface_area, cx);
        }
        if cx.visible(PanelId::Traffic) {
            traffic_table(frame, traffic_area, cx);
        }
    }
}

fn rate_cards(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    let app = cx.app;
    let palette = cx.palette;
    let selected = app.selected_interface();
    let columns =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
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
}

fn interface_table(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    let app = cx.app;
    let palette = cx.palette;
    let selected = app.selected_interface();
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
            .border_style(border_for(cx, PanelId::Interfaces))
            .style(Style::default().bg(palette.surface)),
    )
    .style(Style::default().fg(palette.text))
    .row_highlight_style(highlight(&palette))
    .highlight_symbol("▸ ")
    .column_spacing(1);
    frame.render_stateful_widget(
        interfaces_table,
        area,
        &mut table_state(app, PanelId::Interfaces, app.state.interfaces.len()),
    );
    cx.out
        .viewports
        .insert(PanelId::Interfaces, area.height.saturating_sub(3) as usize);
}

fn traffic_table(frame: &mut Frame, area: Rect, cx: &mut RenderCx) {
    let app = cx.app;
    let palette = cx.palette;
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
            .border_style(border_for(cx, PanelId::Traffic))
            .style(Style::default().bg(palette.surface)),
    )
    .style(Style::default().fg(palette.text))
    .row_highlight_style(highlight(&palette))
    .highlight_symbol("▸ ");
    frame.render_stateful_widget(
        traffic_table,
        area,
        &mut table_state(app, PanelId::Traffic, talkers.len()),
    );
    cx.out
        .viewports
        .insert(PanelId::Traffic, area.height.saturating_sub(3) as usize);
}

fn rate_or_na(value: Option<f64>) -> String {
    value.map(rate).unwrap_or_else(|| "N/A".into())
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
