use crate::model::ConfirmedAction;
use crate::ui::state::{PanelId, Workspace};

/// A semantic user intent, independent of the key or mouse gesture that produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Quit,
    Refresh,
    GoWorkspace(Workspace),
    NextWorkspace,
    PrevWorkspace,
    FocusNext,
    FocusPrev,
    Focus(PanelId),
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Home,
    End,
    Scroll(PanelId, i16),
    Select(PanelId, usize),
    Open,
    Back,
    StartFilter,
    FilterInput(char),
    FilterBackspace,
    FilterSubmit,
    CycleSort,
    Terminate,
    Kill,
    Confirm,
    Cancel,
    ToggleMenu,
    CycleTheme,
    CycleDensity,
    TogglePanel,
}

/// Work the event loop performs outside the pure `App::update`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Quit,
    Refresh,
    SendSignal(ConfirmedAction),
}
