use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind};

use crate::app::Panel;

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

    pub fn from_number(code: KeyCode) -> Option<Self> {
        Some(match code {
            KeyCode::Char('1') => Self::Overview,
            KeyCode::Char('2') => Self::Processes,
            KeyCode::Char('3') => Self::Network,
            KeyCode::Char('4') => Self::Disks,
            KeyCode::Char('5') => Self::More,
            _ => return None,
        })
    }

    pub fn step(self, direction: i8) -> Self {
        let current = Self::ALL.iter().position(|item| *item == self).unwrap_or(0);
        let next = (current as i8 + direction).rem_euclid(Self::ALL.len() as i8) as usize;
        Self::ALL[next]
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
pub enum UiCommand {
    WorkspaceChanged(Workspace),
    FocusNext,
    TogglePanel(Panel),
    DensityChanged(LayoutDensity),
    Scroll(Panel, i16),
    OpenMenu,
    Noop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiState {
    pub workspace: Workspace,
    pub density: LayoutDensity,
    pub hidden_panels: HashSet<Panel>,
    pub scroll_offsets: HashMap<Panel, u16>,
    pub menu_open: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            workspace: Workspace::Overview,
            density: LayoutDensity::Balanced,
            hidden_panels: HashSet::new(),
            scroll_offsets: HashMap::new(),
            menu_open: false,
        }
    }
}

impl UiState {
    pub fn handle_key(&mut self, key: KeyEvent, focused: Panel) -> UiCommand {
        if let Some(workspace) = Workspace::from_number(key.code) {
            self.workspace = workspace;
            self.menu_open = false;
            return UiCommand::WorkspaceChanged(workspace);
        }

        match key.code {
            KeyCode::Left => {
                self.workspace = self.workspace.step(-1);
                UiCommand::WorkspaceChanged(self.workspace)
            }
            KeyCode::Right => {
                self.workspace = self.workspace.step(1);
                UiCommand::WorkspaceChanged(self.workspace)
            }
            KeyCode::Tab => UiCommand::FocusNext,
            KeyCode::Char('h' | 'H') => {
                if self.hidden_panels.remove(&focused) {
                    UiCommand::TogglePanel(focused)
                } else {
                    self.hidden_panels.insert(focused);
                    UiCommand::TogglePanel(focused)
                }
            }
            KeyCode::Char('l' | 'L') => {
                self.density = self.density.cycle();
                UiCommand::DensityChanged(self.density)
            }
            KeyCode::Char('m' | 'M') => {
                self.menu_open = !self.menu_open;
                UiCommand::OpenMenu
            }
            KeyCode::PageUp => UiCommand::Scroll(focused, -8),
            KeyCode::PageDown => UiCommand::Scroll(focused, 8),
            _ => UiCommand::Noop,
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, focused: Panel) -> UiCommand {
        let amount = match mouse.kind {
            MouseEventKind::ScrollUp => -3,
            MouseEventKind::ScrollDown => 3,
            _ => return UiCommand::Noop,
        };
        UiCommand::Scroll(focused, amount)
    }

    pub fn scroll_by(&mut self, panel: Panel, amount: i16) {
        let offset = self.scroll_offsets.entry(panel).or_default();
        if amount.is_negative() {
            *offset = offset.saturating_sub(amount.unsigned_abs());
        } else {
            *offset = offset.saturating_add(amount as u16);
        }
    }
}
