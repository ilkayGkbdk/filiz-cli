pub mod format;
pub mod state;
pub mod theme;
pub mod widgets;

use std::collections::HashMap;

use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::app::App;
use crate::model::AppMode;
use state::{PanelId, Workspace};

/// Layout facts from the last frame that input handling needs.
#[derive(Clone, Debug, Default)]
pub struct RenderOutput {
    pub viewports: HashMap<PanelId, usize>,
}

pub fn render(frame: &mut Frame, app: &App) -> RenderOutput {
    let mut output = RenderOutput::default();
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(app.ui.theme.palette().bg)),
        area,
    );
    let (header, resource, detail, footer) = match app.ui.density {
        state::LayoutDensity::Compact => (2, 7, 3, 2),
        state::LayoutDensity::Balanced => {
            if area.height >= 28 {
                (3, 12, 6, 2)
            } else if area.height >= 20 {
                (2, 8, 3, 2)
            } else if area.height >= 17 {
                (2, 6, 3, 2)
            } else {
                (1, 2, 0, 1)
            }
        }
        state::LayoutDensity::Spacious => (3, 15, 8, 2),
    };
    let resource = if app.ui.hidden_panels.contains(&PanelId::Resources) {
        0
    } else {
        resource
    };
    let detail = if app.ui.hidden_panels.contains(&PanelId::Details) {
        0
    } else {
        detail
    };
    let process_constraint = if app.ui.hidden_panels.contains(&PanelId::Processes) {
        Constraint::Length(0)
    } else {
        Constraint::Min(0)
    };
    let sections = Layout::vertical([
        Constraint::Length(header),
        Constraint::Length(resource),
        process_constraint,
        Constraint::Length(detail),
        Constraint::Length(footer),
    ])
    .split(area);
    widgets::status(frame, sections[0], app);
    if matches!(
        app.ui.workspace,
        Workspace::Network | Workspace::Disks | Workspace::More
    ) {
        match app.ui.workspace {
            Workspace::Network => widgets::network(
                frame,
                workspace_area(sections[1], sections[2]),
                app,
                &mut output,
            ),
            Workspace::Disks => widgets::disks(
                frame,
                workspace_area(sections[1], sections[2]),
                app,
                &mut output,
            ),
            Workspace::More => widgets::more(frame, workspace_area(sections[1], sections[2]), app),
            _ => unreachable!(),
        }
        widgets::footer(frame, sections[4], app);
        match app.mode {
            AppMode::ProcessDetail => widgets::detail_modal(frame, area, app),
            AppMode::ConfirmingAction => widgets::confirmation_modal(frame, area, app),
            _ => {}
        }
        return output;
    }
    widgets::resources(frame, sections[1], app);
    widgets::processes(frame, sections[2], app, &mut output);
    widgets::details(frame, sections[3], app);
    widgets::footer(frame, sections[4], app);
    match app.mode {
        AppMode::ProcessDetail => widgets::detail_modal(frame, area, app),
        AppMode::ConfirmingAction => widgets::confirmation_modal(frame, area, app),
        _ => {}
    }
    output
}

fn workspace_area(
    primary: ratatui::layout::Rect,
    secondary: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        y: primary.y,
        height: primary.height.saturating_add(secondary.height),
        ..primary
    }
}
