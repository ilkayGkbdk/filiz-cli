mod disks;
mod more;
mod network;
mod overview;
mod processes;

use ratatui::{layout::Rect, Frame};

use super::state::Workspace;
use super::RenderCx;

pub trait WorkspaceView {
    fn render(&self, frame: &mut Frame, area: Rect, cx: &mut RenderCx);
}

pub fn view(workspace: Workspace) -> &'static dyn WorkspaceView {
    match workspace {
        Workspace::Overview => &overview::Overview,
        Workspace::Processes => &processes::Processes,
        Workspace::Network => &network::Network,
        Workspace::Disks => &disks::Disks,
        Workspace::More => &more::More,
    }
}
