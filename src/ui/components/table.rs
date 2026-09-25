use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::TableState;

use crate::app::App;
use crate::ui::hit::HitTarget;
use crate::ui::state::PanelId;
use crate::ui::theme::Palette;
use crate::ui::RenderCx;

pub fn table_state(app: &App, panel: PanelId, len: usize) -> TableState {
    let list = app.ui.list(panel);
    let state = TableState::default().with_offset(list.offset.min(len.saturating_sub(1)));
    if len == 0 {
        state
    } else {
        state.with_selected(Some(list.selected.min(len - 1)))
    }
}

pub fn border_for(cx: &RenderCx, panel: PanelId) -> Style {
    Style::default().fg(if cx.focused(panel) {
        cx.palette.border_focus
    } else {
        cx.palette.border
    })
}

pub fn highlight(palette: &Palette) -> Style {
    Style::default()
        .fg(palette.selection_fg)
        .bg(palette.selection_bg)
        .add_modifier(Modifier::BOLD)
}

/// Register a bordered table panel and its visible rows (1 border + 1 header line above rows).
pub fn register_rows(cx: &mut RenderCx, panel: PanelId, area: Rect, len: usize) {
    cx.out.hits.push(area, HitTarget::Panel(panel));
    let viewport = area.height.saturating_sub(3) as usize;
    let offset = cx.app.ui.list(panel).offset.min(len.saturating_sub(1));
    for (row, index) in (offset..len).take(viewport).enumerate() {
        cx.out.hits.push(
            Rect {
                x: area.x + 1,
                y: area.y + 2 + row as u16,
                width: area.width.saturating_sub(2),
                height: 1,
            },
            HitTarget::Row(panel, index),
        );
    }
}
