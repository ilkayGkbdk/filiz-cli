use std::collections::{HashMap, HashSet};

use super::theme::Theme;
use crate::input::list::ListState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Workspace {
    Overview,
    Processes,
    Network,
    Disks,
    More,
}

impl Workspace {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Processes,
        Self::Network,
        Self::Disks,
        Self::More,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Processes => "Processes",
            Self::Network => "Network",
            Self::Disks => "Disks",
            Self::More => "More…",
        }
    }

    pub fn step(self, direction: i8) -> Self {
        let current = Self::ALL.iter().position(|item| *item == self).unwrap_or(0);
        let next = (current as i8 + direction).rem_euclid(Self::ALL.len() as i8) as usize;
        Self::ALL[next]
    }

    pub fn panels(self) -> &'static [PanelId] {
        match self {
            Self::Overview => &[PanelId::Resources, PanelId::Processes, PanelId::Details],
            Self::Processes => &[PanelId::Processes, PanelId::Details],
            Self::Network => &[PanelId::Interfaces, PanelId::Traffic],
            Self::Disks => &[PanelId::Disks],
            Self::More => &[PanelId::Settings],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PanelId {
    Resources,
    Processes,
    Details,
    Interfaces,
    Traffic,
    Disks,
    Settings,
}

impl PanelId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Resources => "Resources",
            Self::Processes => "Processes",
            Self::Details => "Details",
            Self::Interfaces => "Interfaces",
            Self::Traffic => "Traffic",
            Self::Disks => "Disks",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutDensity {
    Compact,
    Balanced,
    Spacious,
}

impl LayoutDensity {
    pub fn cycle(self) -> Self {
        match self {
            Self::Compact => Self::Balanced,
            Self::Balanced => Self::Spacious,
            Self::Spacious => Self::Compact,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelToggle {
    Hidden(PanelId),
    Restored,
    LastPanel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiState {
    pub workspace: Workspace,
    pub focus: PanelId,
    pub density: LayoutDensity,
    pub hidden_panels: HashSet<PanelId>,
    pub lists: HashMap<PanelId, ListState>,
    pub menu_open: bool,
    pub theme: Theme,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            workspace: Workspace::Overview,
            focus: PanelId::Processes,
            density: LayoutDensity::Balanced,
            hidden_panels: HashSet::new(),
            lists: HashMap::new(),
            menu_open: false,
            theme: Theme::Forest,
        }
    }
}

impl UiState {
    pub fn visible_panels(&self) -> Vec<PanelId> {
        self.workspace
            .panels()
            .iter()
            .copied()
            .filter(|panel| !self.hidden_panels.contains(panel))
            .collect()
    }

    pub fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
        self.menu_open = false;
        let visible = self.visible_panels();
        if !visible.contains(&self.focus) {
            self.focus = visible.first().copied().unwrap_or(workspace.panels()[0]);
        }
    }

    pub fn focus_step(&mut self, direction: isize) {
        let visible = self.visible_panels();
        if visible.is_empty() {
            return;
        }
        let current = visible
            .iter()
            .position(|panel| *panel == self.focus)
            .unwrap_or(0) as isize;
        let next = (current + direction).rem_euclid(visible.len() as isize) as usize;
        self.focus = visible[next];
    }

    /// `H`: restore this workspace's hidden panels, or hide the focused one.
    pub fn toggle_panel(&mut self) -> PanelToggle {
        let panels = self.workspace.panels();
        if panels
            .iter()
            .any(|panel| self.hidden_panels.contains(panel))
        {
            for panel in panels {
                self.hidden_panels.remove(panel);
            }
            return PanelToggle::Restored;
        }
        if self.visible_panels().len() <= 1 {
            return PanelToggle::LastPanel;
        }
        let hidden = self.focus;
        self.hidden_panels.insert(hidden);
        self.focus = self.visible_panels()[0];
        PanelToggle::Hidden(hidden)
    }

    pub fn list(&self, panel: PanelId) -> ListState {
        self.lists.get(&panel).copied().unwrap_or_default()
    }

    pub fn list_mut(&mut self, panel: PanelId) -> &mut ListState {
        self.lists.entry(panel).or_default()
    }
}
