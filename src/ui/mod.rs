pub mod components;
pub mod format;
pub mod state;
pub mod theme;
pub mod workspaces;

use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::app::App;
use crate::model::AppMode;
use state::{LayoutDensity, PanelId};
use theme::Palette;

/// Layout facts from the last frame that input handling needs.
#[derive(Clone, Debug, Default)]
pub struct RenderOutput {
    pub viewports: HashMap<PanelId, usize>,
}

pub struct RenderCx<'a> {
    pub app: &'a App,
    pub palette: Palette,
    pub frame_height: u16,
    pub out: &'a mut RenderOutput,
}

impl RenderCx<'_> {
    pub fn visible(&self, panel: PanelId) -> bool {
        !self.app.ui.hidden_panels.contains(&panel)
    }

    pub fn focused(&self, panel: PanelId) -> bool {
        self.app.ui.focus == panel
    }
}

pub fn render(frame: &mut Frame, app: &App) -> RenderOutput {
    let mut output = RenderOutput::default();
    let area = frame.area();
    let palette = app.ui.theme.palette();
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.bg)),
        area,
    );
    let header = match app.ui.density {
        LayoutDensity::Compact => 2,
        LayoutDensity::Spacious => 3,
        LayoutDensity::Balanced => match area.height {
            28.. => 3,
            17..=27 => 2,
            _ => 1,
        },
    };
    let footer = if area.height >= 17 { 2 } else { 1 };
    let [header_area, body, footer_area] = Layout::vertical([
        Constraint::Length(header),
        Constraint::Min(0),
        Constraint::Length(footer),
    ])
    .areas(area);
    let mut cx = RenderCx {
        app,
        palette,
        frame_height: area.height,
        out: &mut output,
    };
    components::status::render(frame, header_area, &mut cx);
    workspaces::view(app.ui.workspace).render(frame, body, &mut cx);
    components::footer::render(frame, footer_area, &mut cx);
    match app.mode {
        AppMode::ProcessDetail => components::modal::detail(frame, area, &mut cx),
        AppMode::ConfirmingAction => components::modal::confirmation(frame, area, &mut cx),
        AppMode::Dashboard | AppMode::Filtering => {}
    }
    output
}
