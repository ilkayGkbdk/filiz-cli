pub mod state;
pub mod theme;
pub mod widgets;

use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
    Frame,
};

use crate::app::App;
use crate::model::AppMode;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::BACKGROUND)),
        area,
    );
    let (header, resource, detail, footer) = match app.ui.density {
        state::LayoutDensity::Compact => (2, 7, 3, 2),
        state::LayoutDensity::Balanced => {
            if area.height >= 28 {
                (3, 12, 6, 2)
            } else if area.height >= 20 {
                (2, 8, 3, 2)
            } else if area.height >= 17 {
                (2, 6, 3, 2)
            } else {
                (1, 2, 0, 1)
            }
        }
        state::LayoutDensity::Spacious => (3, 15, 8, 2),
    };
    let resource = if app.ui.hidden_panels.contains(&crate::app::Panel::Resources) {
        0
    } else {
        resource
    };
    let detail = if app.ui.hidden_panels.contains(&crate::app::Panel::Details) {
        0
    } else {
        detail
    };
    let process_constraint = if app.ui.hidden_panels.contains(&crate::app::Panel::Processes) {
        Constraint::Length(0)
    } else {
        Constraint::Min(0)
    };
    let sections = Layout::vertical([
        Constraint::Length(header),
        Constraint::Length(resource),
        process_constraint,
        Constraint::Length(detail),
        Constraint::Length(footer),
    ])
    .split(area);
    widgets::status(frame, sections[0], app);
    widgets::resources(frame, sections[1], app);
    widgets::processes(frame, sections[2], app);
    widgets::details(frame, sections[3], app);
    widgets::footer(frame, sections[4], app);
    match app.mode {
        AppMode::ProcessDetail => widgets::detail_modal(frame, area, app),
        AppMode::ConfirmingAction => widgets::confirmation_modal(frame, area, app),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProcessIdentity, ProcessInfo, ResourceMetric, SystemSnapshot};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};
    use std::time::{Duration, SystemTime};

    fn sample_app() -> App {
        let mut app = App::new(Duration::from_secs(2));
        app.replace_snapshot(SystemSnapshot {
            captured_at: SystemTime::now(),
            metrics: vec![
                metric("cpu.usage", 27.0, "%"),
                metric("memory.usage", 62.0, "%"),
                metric("memory.used", 16.0 * 1024.0 * 1024.0 * 1024.0, "B"),
                metric("memory.total", 32.0 * 1024.0 * 1024.0 * 1024.0, "B"),
                metric("disk./.usage", 74.0, "%"),
                metric("network.en0.received", 1024.0, "B/s"),
                metric("network.en0.transmitted", 512.0, "B/s"),
                metric("uptime", 3600.0, "s"),
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

    fn metric(name: &str, value: f64, unit: &str) -> ResourceMetric {
        ResourceMetric {
            name: name.into(),
            value: Some(value),
            unit: unit.into(),
        }
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
            .collect::<String>()
    }

    #[test]
    fn dashboard_shows_resources_processes_and_controls() {
        let output = screen(&sample_app(), 110, 35);
        for label in [
            "FILIZ",
            "CPU",
            "MEMORY",
            "DISK",
            "NETWORK",
            "PROCESSES",
            "example-worker",
            "QUIT",
        ] {
            assert!(output.contains(label), "missing {label}");
        }
    }

    #[test]
    fn narrow_terminal_keeps_key_information_visible() {
        let output = screen(&sample_app(), 48, 20);
        for label in [
            "FILIZ",
            "CPU",
            "MEMORY",
            "PROCESSES",
            "example-worker",
            "↑",
            "QUIT",
        ] {
            assert!(output.contains(label), "missing {label}");
        }
        let _ = screen(&sample_app(), 20, 8);
    }

    #[test]
    fn confirmation_modal_names_target_and_requires_yes() {
        let mut app = sample_app();
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        let output = screen(&app, 90, 30);
        for label in ["CONFIRM", "example-worker", "42", "Y", "N"] {
            assert!(output.contains(label), "missing {label}");
        }
    }
}
