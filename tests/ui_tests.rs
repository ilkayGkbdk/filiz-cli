use std::time::{Duration, SystemTime};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use filiz::app::App;
use filiz::model::{ProcessIdentity, ProcessInfo, ResourceMetric, SystemSnapshot};
use filiz::ui::{
    render,
    state::{LayoutDensity, UiCommand, UiState, Workspace},
};
use ratatui::{backend::TestBackend, Terminal};

fn sample_app() -> App {
    let mut app = App::new(Duration::from_secs(2));
    app.replace_snapshot(SystemSnapshot {
        captured_at: SystemTime::now(),
        metrics: vec![
            ResourceMetric {
                name: "cpu.usage".into(),
                value: Some(35.0),
                unit: "%".into(),
            },
            ResourceMetric {
                name: "memory.usage".into(),
                value: Some(62.0),
                unit: "%".into(),
            },
            ResourceMetric {
                name: "disk./.usage".into(),
                value: Some(91.0),
                unit: "%".into(),
            },
        ],
        processes: vec![ProcessInfo {
            identity: ProcessIdentity {
                pid: 42,
                start_time: 100,
            },
            name: "example-worker".into(),
            command: "/usr/bin/example-worker".into(),
            cpu_percent: Some(12.5),
            memory_bytes: Some(1024 * 1024),
            user: Some("user".into()),
            status: Some("Running".into()),
        }],
        events: Vec::new(),
        warnings: Vec::new(),
    });
    app
}

fn screen(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| render(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn dashboard_keeps_core_labels_at_normal_size() {
    let output = screen(&sample_app(), 110, 35);
    for label in [
        "FILIZ",
        "CPU",
        "MEMORY",
        "DISK",
        "NETWORK",
        "PROCESSES",
        "example-worker",
    ] {
        assert!(output.contains(label), "missing {label}");
    }
}

#[test]
fn dashboard_renders_without_panic_when_compact() {
    let output = screen(&sample_app(), 48, 20);
    assert!(output.contains("FILIZ"));
    assert!(output.contains("PROCESSES"));
    let _ = screen(&sample_app(), 20, 8);
}

#[test]
fn confirmation_modal_is_rendered_for_selected_process() {
    let mut app = sample_app();
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    let output = screen(&app, 90, 30);
    assert!(output.contains("CONFIRM"));
    assert!(output.contains("example-worker"));
    assert!(output.contains("Y"));
}

#[test]
fn navigation_switches_workspaces_and_cycles_density() {
    let mut state = UiState::default();
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    assert_eq!(
        state.handle_key(key(KeyCode::Char('3')), filiz::app::Panel::Processes),
        UiCommand::WorkspaceChanged(Workspace::Network)
    );
    assert_eq!(state.workspace, Workspace::Network);
    assert_eq!(
        state.handle_key(key(KeyCode::Char('l')), filiz::app::Panel::Processes),
        UiCommand::DensityChanged(LayoutDensity::Spacious)
    );
    assert_eq!(state.density, LayoutDensity::Spacious);
}

#[test]
fn panel_toggle_and_scroll_are_scoped_to_the_focused_panel() {
    let mut state = UiState::default();
    let key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    assert_eq!(
        state.handle_key(key, filiz::app::Panel::Processes),
        UiCommand::TogglePanel(filiz::app::Panel::Processes)
    );
    assert!(state.hidden_panels.contains(&filiz::app::Panel::Processes));
    state.scroll_by(filiz::app::Panel::Processes, 6);
    assert_eq!(
        state.scroll_offsets.get(&filiz::app::Panel::Processes),
        Some(&6)
    );
    assert_eq!(state.scroll_offsets.get(&filiz::app::Panel::Details), None);
}
