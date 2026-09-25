use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use filiz::app::App;
use filiz::input::action::Effect;
use filiz::model::{ActionKind, AppMode, PendingAction, ProcessIdentity, ProcessInfo, SortMode};
use filiz::state::{CollectorUpdate, InterfaceStats, SystemSample};
use filiz::ui::state::{PanelId, Workspace};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn process(pid: u32, cpu: f32) -> ProcessInfo {
    ProcessInfo {
        identity: ProcessIdentity {
            pid,
            start_time: u64::from(pid) * 100,
        },
        name: format!("process-{pid}"),
        command: format!("/bin/process-{pid}"),
        cpu_percent: Some(cpu),
        memory_bytes: Some(u64::from(pid) * 1024),
        user: None,
        status: None,
        traffic: None,
    }
}

fn iface(name: &str) -> InterfaceStats {
    InterfaceStats {
        name: name.into(),
        rx_rate: Some(1.0),
        tx_rate: Some(1.0),
        rx_total: 0,
        tx_total: 0,
        peak_rx: 0.0,
        peak_tx: 0.0,
    }
}

fn app_with(processes: Vec<ProcessInfo>, interfaces: Vec<InterfaceStats>) -> App {
    let mut app = App::new(Duration::from_secs(2));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes,
        interfaces,
        ..Default::default()
    }));
    app
}

fn app_with_processes() -> App {
    app_with(vec![process(20, 10.0), process(10, 20.0)], Vec::new())
}

#[test]
fn q_and_ctrl_c_quit() {
    let mut app = app_with_processes();
    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), vec![Effect::Quit]);
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        vec![Effect::Quit]
    );
}

#[test]
fn tab_cycles_only_visible_panels_of_current_workspace() {
    let mut app = app_with_processes();
    assert_eq!(app.ui.focus, PanelId::Processes);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Details);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Resources);
    app.handle_key(key(KeyCode::Char('3')));
    assert_eq!(app.ui.focus, PanelId::Interfaces);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Traffic);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.ui.focus, PanelId::Interfaces);
}

#[test]
fn hiding_a_panel_moves_focus_and_h_restores() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Tab));
    app.handle_key(key(KeyCode::Char('h')));
    assert!(app.ui.hidden_panels.contains(&PanelId::Details));
    assert_ne!(app.ui.focus, PanelId::Details);
    for _ in 0..3 {
        app.handle_key(key(KeyCode::Tab));
        assert_ne!(app.ui.focus, PanelId::Details);
    }
    app.handle_key(key(KeyCode::Char('h')));
    assert!(app.ui.hidden_panels.is_empty());
}

#[test]
fn last_visible_panel_cannot_be_hidden() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('4')));
    app.handle_key(key(KeyCode::Char('h')));
    assert!(app.ui.hidden_panels.is_empty());
    assert!(app.notice.as_deref().unwrap().contains("cannot be hidden"));
}

#[test]
fn arrows_select_sorted_processes_without_leaving_bounds() {
    let mut app = app_with_processes();
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_process().unwrap().identity.pid, 20);
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_process().unwrap().identity.pid, 20);
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
}

#[test]
fn page_down_moves_by_rendered_viewport() {
    let processes = (1..=20).map(|pid| process(pid, 1.0)).collect();
    let mut app = app_with(processes, Vec::new());
    app.last_render.viewports.insert(PanelId::Processes, 5);
    app.handle_key(key(KeyCode::PageDown));
    assert_eq!(app.ui.list(PanelId::Processes).selected, 5);
    app.handle_key(key(KeyCode::End));
    assert_eq!(app.ui.list(PanelId::Processes).selected, 19);
    assert_eq!(app.ui.list(PanelId::Processes).offset, 15);
    app.handle_key(key(KeyCode::Home));
    assert_eq!(app.ui.list(PanelId::Processes), Default::default());
}

#[test]
fn m_toggles_menu_without_changing_sort_and_s_cycles_sort() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('m')));
    assert!(app.ui.menu_open);
    assert_eq!(app.sort, SortMode::Cpu);
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.ui.menu_open);
    app.handle_key(key(KeyCode::Char('s')));
    assert_eq!(app.sort, SortMode::Memory);
    assert_eq!(app.visible_processes()[0].identity.pid, 20);
    assert_eq!(
        app.selected_process().unwrap().identity.pid,
        10,
        "selection follows identity"
    );
}

#[test]
fn enter_opens_detail_and_escape_closes_it() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.mode, AppMode::ProcessDetail);
    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.mode, AppMode::Dashboard);
}

#[test]
fn confirmation_requires_explicit_yes() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.mode, AppMode::ConfirmingAction);
    assert!(app.handle_key(key(KeyCode::Enter)).is_empty());
    assert!(app.handle_key(key(KeyCode::Char('q'))).is_empty());
    assert_eq!(
        app.handle_key(key(KeyCode::Char('y'))),
        vec![Effect::SendSignal(
            PendingAction::new(process(10, 20.0).identity, ActionKind::Terminate).confirm()
        )]
    );
    assert_eq!(app.mode, AppMode::Dashboard);
}

#[test]
fn shift_k_requests_kill_and_n_cancels() {
    let mut app = app_with_processes();
    app.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
    assert_eq!(app.pending_action.unwrap().kind(), ActionKind::Kill);
    assert!(app.handle_key(key(KeyCode::Char('n'))).is_empty());
    assert!(app.pending_action.is_none());
    assert_eq!(app.mode, AppMode::Dashboard);
}

#[test]
fn filter_mode_keeps_typed_text_including_q() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('f')));
    for c in ['q', '2', 'İ'] {
        assert!(app.handle_key(key(KeyCode::Char(c))).is_empty());
    }
    assert_eq!(app.filter, "q2İ");
    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Backspace));
    app.handle_key(key(KeyCode::Char('2')));
    assert_eq!(app.visible_processes().len(), 1);
    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.mode, AppMode::Dashboard);
    app.handle_key(key(KeyCode::Esc));
    assert!(app.filter.is_empty());
}

#[test]
fn network_arrows_move_interfaces_not_processes() {
    let mut app = app_with(
        vec![process(20, 10.0), process(10, 20.0)],
        vec![iface("en0"), iface("en1"), iface("utun3")],
    );
    app.handle_key(key(KeyCode::Char('3')));
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.selected_interface().unwrap().name, "utun3");
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.selected_interface().unwrap().name, "en1");
}

#[test]
fn interface_selection_clamps_when_interface_disappears() {
    let mut app = app_with(Vec::new(), vec![iface("en0"), iface("en1"), iface("utun3")]);
    app.update(filiz::input::action::Action::GoWorkspace(
        Workspace::Network,
    ));
    app.handle_key(key(KeyCode::End));
    app.apply_update(CollectorUpdate::System(SystemSample {
        interfaces: vec![iface("en0")],
        ..Default::default()
    }));
    assert_eq!(app.selected_interface().unwrap().name, "en0");
}

#[test]
fn selection_is_kept_by_full_identity() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Down));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: vec![process(20, 80.0), process(10, 1.0)],
        ..Default::default()
    }));
    assert_eq!(app.selected_process().unwrap().identity.pid, 20);
    let mut reused = process(20, 80.0);
    reused.identity.start_time += 1;
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: vec![reused, process(10, 1.0)],
        ..Default::default()
    }));
    assert_eq!(app.selected_process().unwrap().identity.pid, 10);
}

#[test]
fn process_exit_during_confirmation_cancels_it() {
    let mut app = app_with_processes();
    app.handle_key(key(KeyCode::Char('k')));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: vec![process(20, 10.0)],
        ..Default::default()
    }));
    assert_eq!(app.mode, AppMode::Dashboard);
    assert!(app.pending_action.is_none());
    assert!(app
        .notice
        .as_deref()
        .unwrap()
        .contains("no longer available"));
}

#[test]
fn self_process_does_not_enter_action_confirmation() {
    let mut app = app_with(vec![process(std::process::id(), 1.0)], Vec::new());
    assert!(app.handle_key(key(KeyCode::Char('k'))).is_empty());
    assert_eq!(app.mode, AppMode::Dashboard);
    assert!(app.notice.as_deref().unwrap().contains("itself"));
}

#[test]
fn r_requests_refresh_effect() {
    let mut app = app_with_processes();
    assert_eq!(
        app.handle_key(key(KeyCode::Char('r'))),
        vec![Effect::Refresh]
    );
}
