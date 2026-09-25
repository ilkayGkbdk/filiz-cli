use ratatui::style::{Modifier, Style};
use ratatui::widgets::TableState;

use crate::app::App;
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
