use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

use super::state::{PanelId, Workspace};
use crate::input::action::Action;

const DOUBLE_CLICK: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTarget {
    Tab(Workspace),
    Panel(PanelId),
    Row(PanelId, usize),
    Button(Action),
    Blocker,
}

/// Clickable regions of the last frame; later entries are on top.
#[derive(Clone, Debug, Default)]
pub struct HitMap {
    entries: Vec<(Rect, HitTarget)>,
}

impl HitMap {
    pub fn push(&mut self, area: Rect, target: HitTarget) {
        if area.width > 0 && area.height > 0 {
            self.entries.push((area, target));
        }
    }

    pub fn at(&self, column: u16, row: u16) -> Option<HitTarget> {
        let point = Position::new(column, row);
        self.entries
            .iter()
            .rev()
            .find(|(area, _)| area.contains(point))
            .map(|(_, target)| *target)
    }

    pub fn panel_at(&self, column: u16, row: u16) -> Option<PanelId> {
        let point = Position::new(column, row);
        for (area, target) in self.entries.iter().rev() {
            if !area.contains(point) {
                continue;
            }
            match target {
                HitTarget::Panel(panel) | HitTarget::Row(panel, _) => return Some(*panel),
                HitTarget::Blocker => return None,
                HitTarget::Tab(_) | HitTarget::Button(_) => {}
            }
        }
        None
    }

    pub fn has_blocker(&self) -> bool {
        self.entries
            .iter()
            .any(|(_, target)| *target == HitTarget::Blocker)
    }

    pub fn entries(&self) -> &[(Rect, HitTarget)] {
        &self.entries
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClickMemory {
    last: Option<(PanelId, usize, Instant)>,
}

pub fn mouse_action(
    hits: &HitMap,
    event: &MouseEvent,
    focus: PanelId,
    memory: &mut ClickMemory,
    now: Instant,
) -> Option<Action> {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => match hits.at(event.column, event.row)? {
            HitTarget::Tab(workspace) => Some(Action::GoWorkspace(workspace)),
            HitTarget::Panel(panel) => Some(Action::Focus(panel)),
            HitTarget::Button(action) => Some(action),
            HitTarget::Blocker => None,
            HitTarget::Row(panel, index) => {
                let double = memory.last.is_some_and(|(last_panel, last_index, at)| {
                    last_panel == panel
                        && last_index == index
                        && now.saturating_duration_since(at) <= DOUBLE_CLICK
                });
                memory.last = (!double).then_some((panel, index, now));
                Some(if double {
                    Action::Open
                } else {
                    Action::Select(panel, index)
                })
            }
        },
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            if hits.has_blocker() {
                return None;
            }
            let delta = if event.kind == MouseEventKind::ScrollUp {
                -3
            } else {
                3
            };
            let panel = hits.panel_at(event.column, event.row).unwrap_or(focus);
            Some(Action::Scroll(panel, delta))
        }
        _ => None,
    }
}
