use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use filiz::app::App;
use filiz::model::{ProcessIdentity, ProcessInfo};
use filiz::state::{CollectorUpdate, DiskStats, InterfaceStats, MemoryStats, SystemSample};
use filiz::ui::{
    render,
    state::{PanelId, Workspace},
};
use ratatui::{backend::TestBackend, Terminal};

fn sample_app() -> App {
    let mut app = App::new(Duration::from_secs(2));
    app.apply_update(CollectorUpdate::System(SystemSample {
        cpu_usage: Some(35.0),
        memory: MemoryStats {
            total: Some(32 * 1024 * 1024 * 1024),
            used: Some(20 * 1024 * 1024 * 1024),
            ..Default::default()
        },
        disks: vec![DiskStats {
            mount: "/".into(),
            total: 100,
            used: 91,
            free: 9,
            is_system: false,
        }],
        interfaces: vec![InterfaceStats {
            name: "en0".into(),
            rx_rate: Some(1024.0),
            tx_rate: Some(512.0),
            rx_total: 0,
            tx_total: 0,
            peak_rx: 0.0,
            peak_tx: 0.0,
        }],
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
            traffic: None,
        }],
        uptime: Some(Duration::from_secs(3600)),
        ..Default::default()
    }));
    app
}

fn screen(app: &App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| {
            render(frame, app);
        })
        .unwrap();
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
    assert!(output.contains("QUIT"));
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
fn footer_shows_only_keys_of_the_active_context() {
    let mut app = sample_app();
    let processes = screen(&app, 110, 35);
    assert!(processes.contains("FILTER"));
    app.ui.set_workspace(Workspace::Network);
    let network = screen(&app, 110, 35);
    assert!(!network.contains("FILTER"));
    assert!(network.contains("QUIT"));
    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
    app.ui.focus = PanelId::Interfaces;
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert!(app.pending_action.is_none());
}

#[test]
fn sort_column_is_marked() {
    let mut app = sample_app();
    assert!(screen(&app, 110, 35).contains("CPU ▼"));
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert!(screen(&app, 110, 35).contains("MEMORY ▼"));
}

#[test]
fn every_workspace_renders_at_supported_terminal_sizes() {
    for workspace in Workspace::ALL {
        for (width, height) in [(110, 35), (80, 24), (48, 20), (20, 8)] {
            let mut app = sample_app();
            app.ui.workspace = workspace;
            let output = screen(&app, width, height);
            assert!(
                !output.is_empty(),
                "empty output for {workspace:?} at {width}x{height}"
            );
        }
    }
}

#[test]
fn workspace_content_and_theme_controls_are_visible() {
    let mut app = sample_app();
    app.ui.workspace = Workspace::Network;
    assert!(screen(&app, 110, 35).contains("DOWNLOAD"));
    app.ui.workspace = Workspace::Disks;
    assert!(screen(&app, 110, 35).contains("DISKS"));
    app.ui.workspace = Workspace::More;
    let output = screen(&app, 110, 35);
    assert!(output.contains("MORE / SETTINGS"));
    assert!(output.contains("for betül, with love"));
}

#[test]
fn missing_sensors_keep_status_normal_but_failures_warn() {
    let mut app = sample_app();
    assert!(screen(&app, 110, 35).contains("SYSTEM NORMAL"));
    app.apply_update(CollectorUpdate::Failed {
        source: filiz::state::Source::Platform,
        message: "top failed".into(),
    });
    assert!(screen(&app, 110, 35).contains("CHECK METRICS"));
}

#[test]
fn disks_workspace_hides_system_volumes() {
    let mut app = sample_app();
    let sample = SystemSample {
        disks: vec![
            DiskStats {
                mount: "/".into(),
                total: 100,
                used: 50,
                free: 50,
                is_system: false,
            },
            DiskStats {
                mount: "/System/Volumes/VM".into(),
                total: 100,
                used: 1,
                free: 99,
                is_system: true,
            },
        ],
        ..Default::default()
    };
    app.apply_update(CollectorUpdate::System(sample));
    app.ui.workspace = Workspace::Disks;
    let output = screen(&app, 110, 35);
    assert!(!output.contains("/System/Volumes/VM"));
}
