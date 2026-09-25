use std::time::{Duration, Instant};

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use filiz::app::App;
use filiz::input::action::{Action, Effect};
use filiz::model::{AppMode, ProcessIdentity, ProcessInfo, TrafficRate};
use filiz::state::{CollectorUpdate, ProcessTraffic, SystemSample};
use filiz::ui::hit::{mouse_action, ClickMemory, HitMap, HitTarget};
use filiz::ui::state::{PanelId, Workspace};
use filiz::ui::{render, RenderOutput};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn click(column: u16, row: u16) -> MouseEvent {
    mouse(MouseEventKind::Down(MouseButton::Left), column, row)
}

fn process(pid: u32, cpu: f32, traffic: Option<f64>) -> ProcessInfo {
    ProcessInfo {
        identity: ProcessIdentity { pid, start_time: 1 },
        name: format!("proc-{pid}"),
        command: String::new(),
        cpu_percent: Some(cpu),
        memory_bytes: Some(1),
        user: None,
        status: None,
        traffic: traffic.map(|rx| TrafficRate { rx, tx: 0.0 }),
    }
}

fn app() -> App {
    let mut app = App::new(Duration::from_secs(2));
    app.apply_update(CollectorUpdate::System(SystemSample {
        processes: (1..=30)
            .map(|pid| process(pid, 100.0 - pid as f32, Some(pid as f64)))
            .collect(),
        ..Default::default()
    }));
    // `SystemState::apply` re-derives each process's `traffic` field from the
    // dedicated traffic map (populated only by `CollectorUpdate::Traffic`), so the
    // `traffic` values baked into the processes above get overwritten. Feed the
    // same rates through the traffic channel so `traffic_rows()` is non-empty.
    app.apply_update(CollectorUpdate::Traffic(
        (1..=30)
            .map(|pid| ProcessTraffic {
                pid,
                rate: TrafficRate {
                    rx: pid as f64,
                    tx: 0.0,
                },
            })
            .collect(),
    ));
    app
}

fn draw(app: &mut App) {
    let mut terminal = Terminal::new(TestBackend::new(110, 35)).unwrap();
    let mut output = RenderOutput::default();
    terminal.draw(|frame| output = render(frame, app)).unwrap();
    app.last_render = output;
}

fn find(app: &App, target: HitTarget) -> Rect {
    app.last_render
        .hits
        .entries()
        .iter()
        .find(|(_, candidate)| *candidate == target)
        .map(|(rect, _)| *rect)
        .unwrap_or_else(|| panic!("{target:?} not registered"))
}

#[test]
fn later_entries_win_and_blocker_stops_panel_lookup() {
    let mut hits = HitMap::default();
    hits.push(
        Rect::new(0, 0, 10, 10),
        HitTarget::Panel(PanelId::Processes),
    );
    hits.push(
        Rect::new(0, 2, 10, 1),
        HitTarget::Row(PanelId::Processes, 0),
    );
    assert_eq!(hits.at(3, 2), Some(HitTarget::Row(PanelId::Processes, 0)));
    assert_eq!(hits.panel_at(3, 5), Some(PanelId::Processes));
    assert_eq!(hits.at(20, 20), None);
    hits.push(Rect::new(0, 0, 10, 10), HitTarget::Blocker);
    assert_eq!(hits.panel_at(3, 5), None);
    assert!(hits.has_blocker());
}

#[test]
fn second_click_on_same_row_within_400ms_opens() {
    let mut hits = HitMap::default();
    hits.push(
        Rect::new(0, 0, 10, 1),
        HitTarget::Row(PanelId::Processes, 4),
    );
    let mut memory = ClickMemory::default();
    let now = Instant::now();
    let first = mouse_action(&hits, &click(1, 0), PanelId::Resources, &mut memory, now);
    assert_eq!(first, Some(Action::Select(PanelId::Processes, 4)));
    let second = mouse_action(
        &hits,
        &click(1, 0),
        PanelId::Processes,
        &mut memory,
        now + Duration::from_millis(200),
    );
    assert_eq!(second, Some(Action::Open));
    let late = mouse_action(
        &hits,
        &click(1, 0),
        PanelId::Processes,
        &mut memory,
        now + Duration::from_secs(2),
    );
    assert_eq!(late, Some(Action::Select(PanelId::Processes, 4)));
}

#[test]
fn clicking_a_tab_switches_workspace() {
    let mut app = app();
    draw(&mut app);
    let tab = find(&app, HitTarget::Tab(Workspace::Network));
    app.handle_mouse(click(tab.x, tab.y));
    assert_eq!(app.ui.workspace, Workspace::Network);
}

#[test]
fn clicking_rows_selects_and_double_click_opens_detail() {
    let mut app = app();
    draw(&mut app);
    let row = find(&app, HitTarget::Row(PanelId::Processes, 2));
    app.handle_mouse(click(row.x + 1, row.y));
    assert_eq!(app.selected_process().unwrap().identity.pid, 3);
    app.handle_mouse(click(row.x + 1, row.y));
    assert_eq!(app.mode, AppMode::ProcessDetail);
}

#[test]
fn wheel_scrolls_the_panel_under_the_cursor() {
    let mut app = app();
    app.ui.set_workspace(Workspace::Network);
    draw(&mut app);
    assert_eq!(app.ui.focus, PanelId::Interfaces);
    let traffic = find(&app, HitTarget::Panel(PanelId::Traffic));
    app.handle_mouse(mouse(
        MouseEventKind::ScrollDown,
        traffic.x + 2,
        traffic.y + 3,
    ));
    assert_eq!(app.ui.list(PanelId::Traffic).selected, 3);
    assert_eq!(app.ui.list(PanelId::Interfaces).selected, 0);
}

#[test]
fn confirmation_modal_blocks_background_and_exposes_buttons() {
    let mut app = app();
    app.update(Action::Terminate);
    draw(&mut app);
    let tab = find(&app, HitTarget::Tab(Workspace::Disks));
    assert!(app.handle_mouse(click(tab.x, tab.y)).is_empty());
    assert_eq!(app.ui.workspace, Workspace::Overview);
    let cancel = find(&app, HitTarget::Button(Action::Cancel));
    app.handle_mouse(click(cancel.x, cancel.y));
    assert_eq!(app.mode, AppMode::Dashboard);

    app.update(Action::Terminate);
    draw(&mut app);
    let confirm = find(&app, HitTarget::Button(Action::Confirm));
    let effects = app.handle_mouse(click(confirm.x, confirm.y));
    assert!(matches!(effects.as_slice(), [Effect::SendSignal(_)]));
}

#[test]
fn footer_hints_are_buttons() {
    let mut app = app();
    draw(&mut app);
    let quit = find(&app, HitTarget::Button(Action::Quit));
    assert_eq!(app.handle_mouse(click(quit.x, quit.y)), vec![Effect::Quit]);
}
