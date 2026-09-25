use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use filiz::input::action::Action;
use filiz::input::keymap::{hints, resolve, Context, BINDINGS};
use filiz::input::list::ListState;
use filiz::ui::state::{PanelId, Workspace};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shifted(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT)
}

fn dashboard(panel: PanelId, workspace: Workspace) -> Vec<Context> {
    vec![
        Context::Panel(panel),
        Context::Workspace(workspace),
        Context::Global,
    ]
}

#[test]
fn no_context_binds_the_same_key_twice() {
    let mut seen = HashSet::new();
    for binding in BINDINGS {
        assert!(
            seen.insert((
                format!("{:?}", binding.context),
                format!("{:?}", binding.key)
            )),
            "duplicate binding {:?} in {:?}",
            binding.key,
            binding.context
        );
    }
}

#[test]
fn m_opens_menu_and_s_cycles_sort() {
    let stack = dashboard(PanelId::Processes, Workspace::Processes);
    assert_eq!(
        resolve(&stack, &press(KeyCode::Char('m'))),
        Some(Action::ToggleMenu)
    );
    assert_eq!(
        resolve(&stack, &press(KeyCode::Char('s'))),
        Some(Action::CycleSort)
    );
    assert_eq!(resolve(&stack, &press(KeyCode::Char('c'))), None);
}

#[test]
fn process_keys_only_work_when_process_panel_is_focused() {
    let processes = dashboard(PanelId::Processes, Workspace::Overview);
    let network = dashboard(PanelId::Interfaces, Workspace::Network);
    assert_eq!(
        resolve(&processes, &press(KeyCode::Char('k'))),
        Some(Action::Terminate)
    );
    assert_eq!(resolve(&processes, &shifted('K')), Some(Action::Kill));
    assert_eq!(resolve(&network, &press(KeyCode::Char('k'))), None);
    assert_eq!(resolve(&network, &press(KeyCode::Char('f'))), None);
}

#[test]
fn shift_falls_back_to_unshifted_binding() {
    let stack = dashboard(PanelId::Resources, Workspace::Overview);
    assert_eq!(resolve(&stack, &shifted('Q')), Some(Action::Quit));
    assert_eq!(
        resolve(
            &stack,
            &KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        ),
        Some(Action::Quit)
    );
}

#[test]
fn confirm_context_swallows_unbound_keys() {
    let stack = vec![Context::Confirm];
    assert_eq!(resolve(&stack, &press(KeyCode::Char('q'))), None);
    assert_eq!(resolve(&stack, &press(KeyCode::Enter)), None);
    assert_eq!(
        resolve(&stack, &press(KeyCode::Char('y'))),
        Some(Action::Confirm)
    );
    assert_eq!(resolve(&stack, &press(KeyCode::Esc)), Some(Action::Cancel));
}

#[test]
fn filter_accepts_uppercase_and_turkish_characters() {
    let stack = vec![Context::Filter];
    assert_eq!(
        resolve(&stack, &shifted('Q')),
        Some(Action::FilterInput('Q'))
    );
    assert_eq!(
        resolve(&stack, &press(KeyCode::Char('İ'))),
        Some(Action::FilterInput('İ'))
    );
    assert_eq!(
        resolve(&stack, &press(KeyCode::Char('ş'))),
        Some(Action::FilterInput('ş'))
    );
    assert_eq!(
        resolve(&stack, &press(KeyCode::Enter)),
        Some(Action::FilterSubmit)
    );
    assert_eq!(
        resolve(&stack, &press(KeyCode::Backspace)),
        Some(Action::FilterBackspace)
    );
    assert_eq!(resolve(&stack, &press(KeyCode::Tab)), None);
}

#[test]
fn modified_letters_are_not_shortcuts() {
    let stack = vec![Context::Panel(PanelId::Processes), Context::Global];
    assert_eq!(
        resolve(
            &stack,
            &KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL)
        ),
        None
    );
    assert_eq!(
        resolve(
            &stack,
            &KeyEvent::new(KeyCode::Char('r'), KeyModifiers::ALT)
        ),
        None
    );
    assert_eq!(
        resolve(
            &stack,
            &KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
        ),
        Some(Action::Quit)
    );
}

#[test]
fn key_release_events_are_ignored() {
    let mut event = press(KeyCode::Char('q'));
    event.kind = KeyEventKind::Release;
    assert_eq!(resolve(&[Context::Global], &event), None);
}

#[test]
fn hints_follow_the_active_context() {
    let labels = |stack: &[Context]| {
        hints(stack)
            .iter()
            .filter_map(|binding| binding.hint)
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(&[Context::Confirm]), vec!["CONFIRM", "CANCEL"]);
    let processes = labels(&dashboard(PanelId::Processes, Workspace::Processes));
    assert!(processes.contains(&"FILTER"));
    assert!(processes.contains(&"QUIT"));
    let network = labels(&dashboard(PanelId::Interfaces, Workspace::Network));
    assert!(!network.contains(&"FILTER"));
    assert!(network.contains(&"PANEL"));
}

#[test]
fn list_state_moves_within_bounds_and_keeps_selection_visible() {
    let mut list = ListState::default();
    list.move_by(7, 10, 4);
    assert_eq!(
        list,
        ListState {
            selected: 7,
            offset: 4
        }
    );
    list.move_by(100, 10, 4);
    assert_eq!(
        list,
        ListState {
            selected: 9,
            offset: 6
        }
    );
    list.move_by(-100, 10, 4);
    assert_eq!(
        list,
        ListState {
            selected: 0,
            offset: 0
        }
    );
}

#[test]
fn list_state_survives_shrinking_list_and_viewport() {
    let mut list = ListState {
        selected: 40,
        offset: 35,
    };
    list.clamp(5, 1);
    assert_eq!(
        list,
        ListState {
            selected: 4,
            offset: 4
        }
    );
    list.clamp(0, 0);
    assert_eq!(list, ListState::default());
    list.move_by(3, 0, 10);
    assert_eq!(list, ListState::default());
}

#[test]
fn workspaces_declare_their_panels() {
    assert_eq!(
        Workspace::Overview.panels(),
        &[PanelId::Resources, PanelId::Processes, PanelId::Details]
    );
    assert_eq!(
        Workspace::Processes.panels(),
        &[PanelId::Processes, PanelId::Details]
    );
    assert_eq!(
        Workspace::Network.panels(),
        &[PanelId::Interfaces, PanelId::Traffic]
    );
}
