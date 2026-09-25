use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::action::Action;
use crate::ui::state::{PanelId, Workspace};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    Confirm,
    Detail,
    Filter,
    Menu,
    Panel(PanelId),
    Workspace(Workspace),
    Global,
}

impl Context {
    /// Exclusive contexts swallow keys they do not bind.
    pub fn exclusive(self) -> bool {
        matches!(self, Self::Confirm | Self::Filter)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeySpec {
    pub code: KeyCode,
    pub shift: bool,
}

impl KeySpec {
    const fn key(code: KeyCode) -> Self {
        Self { code, shift: false }
    }

    const fn ch(c: char) -> Self {
        Self::key(KeyCode::Char(c))
    }

    const fn shift_ch(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            shift: true,
        }
    }

    /// Letters are normalized to lowercase with `shift` carrying the case.
    pub fn from_event(event: &KeyEvent) -> Self {
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
            KeyCode::Char(c) if c.is_ascii_alphabetic() => Self {
                code: KeyCode::Char(c.to_ascii_lowercase()),
                shift: shift || c.is_ascii_uppercase(),
            },
            code => Self { code, shift },
        }
    }
}

#[derive(Debug)]
pub struct Binding {
    pub context: Context,
    pub key: KeySpec,
    pub action: Action,
    pub label: &'static str,
    pub hint: Option<&'static str>,
}

const fn bind(
    context: Context,
    key: KeySpec,
    action: Action,
    label: &'static str,
    hint: Option<&'static str>,
) -> Binding {
    Binding {
        context,
        key,
        action,
        label,
        hint,
    }
}

use Context::{Confirm, Detail, Filter, Global, Menu};
const PROCESSES: Context = Context::Panel(PanelId::Processes);

pub static BINDINGS: &[Binding] = &[
    bind(
        Confirm,
        KeySpec::ch('y'),
        Action::Confirm,
        "Y",
        Some("CONFIRM"),
    ),
    bind(
        Confirm,
        KeySpec::ch('n'),
        Action::Cancel,
        "N",
        Some("CANCEL"),
    ),
    bind(
        Confirm,
        KeySpec::key(KeyCode::Esc),
        Action::Cancel,
        "ESC",
        None,
    ),
    bind(
        Filter,
        KeySpec::key(KeyCode::Enter),
        Action::FilterSubmit,
        "ENTER",
        Some("DONE"),
    ),
    bind(
        Filter,
        KeySpec::key(KeyCode::Esc),
        Action::FilterSubmit,
        "ESC",
        Some("CLOSE"),
    ),
    bind(
        Filter,
        KeySpec::key(KeyCode::Backspace),
        Action::FilterBackspace,
        "⌫",
        None,
    ),
    bind(
        Detail,
        KeySpec::key(KeyCode::Esc),
        Action::Back,
        "ESC",
        Some("CLOSE"),
    ),
    bind(
        Detail,
        KeySpec::ch('k'),
        Action::Terminate,
        "K",
        Some("TERMINATE"),
    ),
    bind(
        Detail,
        KeySpec::shift_ch('k'),
        Action::Kill,
        "⇧K",
        Some("KILL"),
    ),
    bind(
        Menu,
        KeySpec::key(KeyCode::Esc),
        Action::ToggleMenu,
        "ESC",
        Some("CLOSE MENU"),
    ),
    bind(
        PROCESSES,
        KeySpec::key(KeyCode::Enter),
        Action::Open,
        "ENTER",
        Some("DETAIL"),
    ),
    bind(
        PROCESSES,
        KeySpec::ch('f'),
        Action::StartFilter,
        "F",
        Some("FILTER"),
    ),
    bind(
        PROCESSES,
        KeySpec::ch('s'),
        Action::CycleSort,
        "S",
        Some("SORT"),
    ),
    bind(
        PROCESSES,
        KeySpec::ch('k'),
        Action::Terminate,
        "K",
        Some("TERMINATE"),
    ),
    bind(
        PROCESSES,
        KeySpec::shift_ch('k'),
        Action::Kill,
        "⇧K",
        Some("KILL"),
    ),
    bind(Global, KeySpec::ch('q'), Action::Quit, "Q", Some("QUIT")),
    bind(
        Global,
        KeySpec::key(KeyCode::Tab),
        Action::FocusNext,
        "TAB",
        Some("PANEL"),
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::BackTab),
        Action::FocusPrev,
        "⇧TAB",
        None,
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::Up),
        Action::MoveUp,
        "↑↓",
        Some("MOVE"),
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::Down),
        Action::MoveDown,
        "↓",
        None,
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::PageUp),
        Action::PageUp,
        "PGUP",
        None,
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::PageDown),
        Action::PageDown,
        "PGDN",
        None,
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::Home),
        Action::Home,
        "HOME",
        None,
    ),
    bind(Global, KeySpec::key(KeyCode::End), Action::End, "END", None),
    bind(
        Global,
        KeySpec::key(KeyCode::Left),
        Action::PrevWorkspace,
        "←→",
        Some("WORKSPACE"),
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::Right),
        Action::NextWorkspace,
        "→",
        None,
    ),
    bind(
        Global,
        KeySpec::ch('1'),
        Action::GoWorkspace(Workspace::Overview),
        "1",
        None,
    ),
    bind(
        Global,
        KeySpec::ch('2'),
        Action::GoWorkspace(Workspace::Processes),
        "2",
        None,
    ),
    bind(
        Global,
        KeySpec::ch('3'),
        Action::GoWorkspace(Workspace::Network),
        "3",
        None,
    ),
    bind(
        Global,
        KeySpec::ch('4'),
        Action::GoWorkspace(Workspace::Disks),
        "4",
        None,
    ),
    bind(
        Global,
        KeySpec::ch('5'),
        Action::GoWorkspace(Workspace::More),
        "5",
        None,
    ),
    bind(
        Global,
        KeySpec::key(KeyCode::Esc),
        Action::Back,
        "ESC",
        None,
    ),
    bind(
        Global,
        KeySpec::ch('m'),
        Action::ToggleMenu,
        "M",
        Some("MENU"),
    ),
    bind(
        Global,
        KeySpec::ch('h'),
        Action::TogglePanel,
        "H",
        Some("HIDE"),
    ),
    bind(
        Global,
        KeySpec::ch('l'),
        Action::CycleDensity,
        "L",
        Some("LAYOUT"),
    ),
    bind(
        Global,
        KeySpec::ch('t'),
        Action::CycleTheme,
        "T",
        Some("THEME"),
    ),
    bind(
        Global,
        KeySpec::ch('r'),
        Action::Refresh,
        "R",
        Some("REFRESH"),
    ),
];

fn lookup(context: Context, key: KeySpec) -> Option<&'static Binding> {
    BINDINGS
        .iter()
        .find(|binding| binding.context == context && binding.key == key)
}

pub fn resolve(stack: &[Context], event: &KeyEvent) -> Option<Action> {
    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    let control = event.modifiers.contains(KeyModifiers::CONTROL);
    if control && event.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }
    let alt = event.modifiers.contains(KeyModifiers::ALT);
    if control || alt {
        return None;
    }
    let key = KeySpec::from_event(event);
    for &context in stack {
        if context == Context::Filter {
            if let KeyCode::Char(c) = event.code {
                if !control && !c.is_control() {
                    return Some(Action::FilterInput(c));
                }
            }
        }
        let unshifted = KeySpec {
            shift: false,
            ..key
        };
        let found = lookup(context, key)
            .or_else(|| key.shift.then(|| lookup(context, unshifted)).flatten());
        if let Some(binding) = found {
            return Some(binding.action);
        }
        if context.exclusive() {
            return None;
        }
    }
    None
}

/// Hinted bindings reachable from `stack`, in priority order, without shadowed keys.
pub fn hints(stack: &[Context]) -> Vec<&'static Binding> {
    let mut taken: Vec<KeySpec> = Vec::new();
    let mut result = Vec::new();
    for &context in stack {
        for binding in BINDINGS.iter().filter(|binding| binding.context == context) {
            if taken.contains(&binding.key) {
                continue;
            }
            taken.push(binding.key);
            if binding.hint.is_some() {
                result.push(binding);
            }
        }
        if context.exclusive() {
            break;
        }
    }
    result
}
