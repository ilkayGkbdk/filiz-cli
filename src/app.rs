#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::SeriesKey;
    use crate::state::SystemSample;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

    fn app_with_processes() -> App {
        let mut app = App::new(Duration::from_secs(2));
        app.apply_update(CollectorUpdate::System(SystemSample {
            processes: vec![process(20, 10.0), process(10, 20.0)],
            ..Default::default()
        }));
        app
    }

    #[test]
    fn q_quits_and_tab_changes_focus() {
        let mut app = app_with_processes();
        assert_eq!(app.focus, Panel::Processes);
        assert_eq!(app.handle_key(key(KeyCode::Tab)), AppCommand::Noop);
        assert_eq!(app.focus, Panel::Details);
        assert_eq!(app.handle_key(key(KeyCode::Char('q'))), AppCommand::Quit);
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
    fn enter_opens_detail_and_escape_closes_it() {
        let mut app = app_with_processes();
        assert_eq!(
            app.handle_key(key(KeyCode::Enter)),
            AppCommand::OpenProcess(process(10, 20.0).identity)
        );
        assert_eq!(app.mode, AppMode::ProcessDetail);
        assert_eq!(app.handle_key(key(KeyCode::Esc)), AppCommand::Noop);
        assert_eq!(app.mode, AppMode::Dashboard);
    }

    #[test]
    fn k_requests_confirmation_and_escape_rejects_it() {
        let mut app = app_with_processes();
        assert_eq!(
            app.handle_key(key(KeyCode::Char('k'))),
            AppCommand::BeginAction
        );
        assert_eq!(app.mode, AppMode::ConfirmingAction);
        assert_eq!(app.pending_action.unwrap().kind(), ActionKind::Terminate);
        assert_eq!(app.handle_key(key(KeyCode::Esc)), AppCommand::CancelAction);
        assert_eq!(app.mode, AppMode::Dashboard);
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn confirmation_only_accepts_explicit_yes() {
        let mut app = app_with_processes();
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.handle_key(key(KeyCode::Enter)), AppCommand::Noop);
        assert_eq!(app.mode, AppMode::ConfirmingAction);
        let command = app.handle_key(key(KeyCode::Char('y')));
        assert_eq!(
            command,
            AppCommand::ConfirmAction(
                PendingAction::new(process(10, 20.0).identity, ActionKind::Terminate).confirm()
            )
        );
        assert_eq!(app.mode, AppMode::Dashboard);
        assert!(app.pending_action.is_none());
    }

    #[test]
    fn n_rejects_confirmation_and_filter_mode_accepts_text() {
        let mut app = app_with_processes();
        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(
            app.handle_key(key(KeyCode::Char('n'))),
            AppCommand::CancelAction
        );
        assert_eq!(app.handle_key(key(KeyCode::Char('f'))), AppCommand::Noop);
        assert_eq!(app.mode, AppMode::Filtering);
        app.handle_key(key(KeyCode::Char('2')));
        assert_eq!(app.visible_processes().len(), 1);
        assert_eq!(app.visible_processes()[0].identity.pid, 20);
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.mode, AppMode::Dashboard);
    }

    #[test]
    fn replacing_state_keeps_selection_by_full_identity() {
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
    fn network_history_is_recorded_per_interface() {
        let mut app = app_with_processes();
        app.apply_update(CollectorUpdate::System(SystemSample {
            interfaces: vec![crate::state::InterfaceStats {
                name: "en0".into(),
                rx_rate: Some(200_000.0),
                tx_rate: Some(1_000.0),
                rx_total: 0,
                tx_total: 0,
                peak_rx: 0.0,
                peak_tx: 0.0,
            }],
            ..Default::default()
        }));
        assert_eq!(
            app.history.series(&SeriesKey::NetRx("en0".into())).last(),
            Some(&200_000)
        );
    }

    #[test]
    fn self_process_does_not_enter_action_confirmation() {
        let mut app = app_with_processes();
        let self_pid = std::process::id();
        app.apply_update(CollectorUpdate::System(SystemSample {
            processes: vec![process(self_pid, 1.0)],
            ..Default::default()
        }));

        assert_eq!(app.handle_key(key(KeyCode::Char('k'))), AppCommand::Noop);
        assert_eq!(app.mode, AppMode::Dashboard);
        assert!(app.pending_action.is_none());
        assert!(app.notice.as_deref().unwrap_or_default().contains("itself"));
    }
}
use std::io::Stdout;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::actions::ProcessAction;
use crate::collectors::CollectorSet;
use crate::history::History;
use crate::model::{
    filter_processes, sort_processes, ActionKind, AppMode, ConfirmedAction, PendingAction,
    ProcessIdentity, ProcessInfo, SortMode,
};
use crate::state::{CollectorUpdate, SystemState};
use crate::ui;
use crate::ui::state::{UiCommand, UiState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    Processes,
    Details,
    Resources,
    Network,
    Disks,
    More,
}

impl Panel {
    fn next(self) -> Self {
        match self {
            Self::Processes => Self::Details,
            Self::Details => Self::Resources,
            Self::Resources => Self::Network,
            Self::Network => Self::Disks,
            Self::Disks => Self::More,
            Self::More => Self::Processes,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppCommand {
    Quit,
    Refresh,
    OpenProcess(ProcessIdentity),
    BeginAction,
    ConfirmAction(ConfirmedAction),
    CancelAction,
    Noop,
}

pub struct App {
    pub refresh: Duration,
    pub state: SystemState,
    pub mode: AppMode,
    pub focus: Panel,
    pub sort: SortMode,
    pub filter: String,
    pub selected_index: usize,
    pub pending_action: Option<PendingAction>,
    pub notice: Option<String>,
    pub history: History,
    pub ui: UiState,
    selected_identity: Option<ProcessIdentity>,
    notice_until: Option<Instant>,
}

impl App {
    pub fn new(refresh: Duration) -> Self {
        Self {
            refresh,
            state: SystemState::default(),
            mode: AppMode::Dashboard,
            focus: Panel::Processes,
            sort: SortMode::Cpu,
            filter: String::new(),
            selected_index: 0,
            pending_action: None,
            notice: None,
            history: History::new(60),
            ui: UiState::default(),
            selected_identity: None,
            notice_until: None,
        }
    }

    pub fn apply_update(&mut self, update: CollectorUpdate) {
        let is_system = matches!(update, CollectorUpdate::System(_));
        self.state.apply(update);
        if is_system {
            self.history.record(&self.state);
        }
        self.reconcile_selection();
    }

    pub fn visible_processes(&self) -> Vec<ProcessInfo> {
        let mut processes = filter_processes(&self.state.processes, &self.filter);
        sort_processes(&mut processes, self.sort);
        processes
    }

    pub fn selected_process(&self) -> Option<ProcessInfo> {
        self.visible_processes().get(self.selected_index).cloned()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppCommand {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return AppCommand::Noop;
        }
        match self.mode {
            AppMode::ConfirmingAction => self.handle_confirmation(key.code),
            AppMode::Filtering => self.handle_filter(key.code),
            AppMode::Dashboard | AppMode::ProcessDetail => self.handle_dashboard(key),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if self.mode != AppMode::Dashboard {
            return;
        }
        if let UiCommand::Scroll(panel, amount) = self.ui.handle_mouse(mouse, self.focus) {
            self.ui.scroll_by(panel, amount);
            if panel == Panel::Processes {
                if amount.is_positive() {
                    self.move_selection(amount as usize);
                } else {
                    self.move_selection_up(amount.unsigned_abs() as usize);
                }
            }
        }
    }

    fn handle_confirmation(&mut self, code: KeyCode) -> AppCommand {
        match code {
            KeyCode::Char('y' | 'Y') => {
                self.mode = AppMode::Dashboard;
                self.pending_action
                    .take()
                    .map(|pending| AppCommand::ConfirmAction(pending.confirm()))
                    .unwrap_or(AppCommand::Noop)
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.mode = AppMode::Dashboard;
                if let Some(pending) = self.pending_action.take() {
                    let _ = pending.cancel();
                }
                AppCommand::CancelAction
            }
            _ => AppCommand::Noop,
        }
    }

    fn handle_filter(&mut self, code: KeyCode) -> AppCommand {
        match code {
            KeyCode::Esc | KeyCode::Enter => self.mode = AppMode::Dashboard,
            KeyCode::Backspace => {
                self.filter.pop();
                self.select_first();
            }
            KeyCode::Char(character) if !character.is_control() => {
                self.filter.push(character);
                self.select_first();
            }
            _ => {}
        }
        AppCommand::Noop
    }

    fn handle_dashboard(&mut self, key: KeyEvent) -> AppCommand {
        match self.ui.handle_key(key, self.focus) {
            UiCommand::WorkspaceChanged(workspace) => {
                self.focus = match workspace {
                    crate::ui::state::Workspace::Network => Panel::Network,
                    crate::ui::state::Workspace::Disks => Panel::Disks,
                    crate::ui::state::Workspace::More => Panel::More,
                    crate::ui::state::Workspace::Overview => Panel::Resources,
                    crate::ui::state::Workspace::Processes => Panel::Processes,
                };
                self.show_notice(format!("Workspace: {}", workspace.label()));
                return AppCommand::Noop;
            }
            UiCommand::FocusNext => {
                self.focus = self.focus.next();
                return AppCommand::Noop;
            }
            UiCommand::TogglePanel(panel) => {
                self.show_notice(format!(
                    "{} panel {}.",
                    panel_label(panel),
                    if self.ui.hidden_panels.contains(&panel) {
                        "hidden"
                    } else {
                        "visible"
                    }
                ));
                return AppCommand::Noop;
            }
            UiCommand::DensityChanged(density) => {
                self.show_notice(format!("Layout: {density:?}"));
                return AppCommand::Noop;
            }
            UiCommand::OpenMenu => return AppCommand::Noop,
            UiCommand::NetworkInterfaceChanged(index) => {
                self.show_notice(format!("Network interface: {}", index + 1));
                return AppCommand::Noop;
            }
            UiCommand::Scroll(panel, amount) => {
                self.ui.scroll_by(panel, amount);
                return AppCommand::Noop;
            }
            UiCommand::Noop => {}
        }
        match key.code {
            KeyCode::Char('q' | 'Q') => AppCommand::Quit,
            KeyCode::Tab => {
                self.focus = self.focus.next();
                AppCommand::Noop
            }
            KeyCode::Down | KeyCode::Up => {
                let processes = self.visible_processes();
                if !processes.is_empty() {
                    self.selected_index = if key.code == KeyCode::Down {
                        (self.selected_index + 1).min(processes.len() - 1)
                    } else {
                        self.selected_index.saturating_sub(1)
                    };
                    self.selected_identity = Some(processes[self.selected_index].identity);
                    self.focus = Panel::Processes;
                }
                AppCommand::Noop
            }
            KeyCode::Enter => {
                if let Some(process) = self.selected_process() {
                    self.mode = AppMode::ProcessDetail;
                    AppCommand::OpenProcess(process.identity)
                } else {
                    AppCommand::Noop
                }
            }
            KeyCode::Esc => {
                if self.mode == AppMode::ProcessDetail {
                    self.mode = AppMode::Dashboard;
                } else if !self.filter.is_empty() {
                    self.filter.clear();
                    self.select_first();
                }
                AppCommand::Noop
            }
            KeyCode::Char('k' | 'K') => {
                if let Some(process) = self.selected_process() {
                    if process.identity.pid == std::process::id() {
                        self.show_notice("Filiz cannot send a signal to itself.");
                        return AppCommand::Noop;
                    }
                    let kind = if key.modifiers.contains(KeyModifiers::SHIFT)
                        || key.code == KeyCode::Char('K')
                    {
                        ActionKind::Kill
                    } else {
                        ActionKind::Terminate
                    };
                    self.pending_action = Some(PendingAction::new(process.identity, kind));
                    self.mode = AppMode::ConfirmingAction;
                    AppCommand::BeginAction
                } else {
                    AppCommand::Noop
                }
            }
            KeyCode::Char('f' | 'F') => {
                self.mode = AppMode::Filtering;
                AppCommand::Noop
            }
            KeyCode::Char('c' | 'C') => {
                self.sort = SortMode::Cpu;
                self.reconcile_selection();
                AppCommand::Noop
            }
            KeyCode::Char('m' | 'M') => {
                self.sort = SortMode::Memory;
                self.reconcile_selection();
                AppCommand::Noop
            }
            KeyCode::Char('r' | 'R') => AppCommand::Refresh,
            _ => AppCommand::Noop,
        }
    }

    fn move_selection(&mut self, amount: usize) {
        let processes = self.visible_processes();
        if !processes.is_empty() {
            self.selected_index = (self.selected_index + amount).min(processes.len() - 1);
            self.selected_identity = Some(processes[self.selected_index].identity);
        }
    }

    fn move_selection_up(&mut self, amount: usize) {
        self.selected_index = self.selected_index.saturating_sub(amount);
        self.selected_identity = self.selected_process().map(|process| process.identity);
    }

    fn select_first(&mut self) {
        self.selected_index = 0;
        self.selected_identity = self.visible_processes().first().map(|p| p.identity);
    }

    fn reconcile_selection(&mut self) {
        let processes = self.visible_processes();
        let previous = self.selected_identity;
        self.selected_index = previous
            .and_then(|identity| processes.iter().position(|p| p.identity == identity))
            .or_else(|| {
                processes
                    .iter()
                    .position(|p| previous.is_none_or(|identity| p.identity.pid != identity.pid))
            })
            .unwrap_or(0);
        self.selected_identity = processes.get(self.selected_index).map(|p| p.identity);
        if self.mode == AppMode::ConfirmingAction
            && self
                .pending_action
                .is_some_and(|pending| !processes.iter().any(|p| p.identity == pending.identity()))
        {
            self.pending_action = None;
            self.mode = AppMode::Dashboard;
            self.show_notice("Selected process is no longer available.");
        }
        if self.mode == AppMode::ProcessDetail && self.selected_identity != previous {
            self.mode = AppMode::Dashboard;
        }
    }

    pub fn show_notice(&mut self, message: impl Into<String>) {
        self.notice = Some(message.into());
        self.notice_until = Some(Instant::now() + Duration::from_secs(5));
    }

    fn clear_expired_notice(&mut self) -> bool {
        if self
            .notice_until
            .is_some_and(|until| Instant::now() >= until)
        {
            self.notice = None;
            self.notice_until = None;
            return true;
        }
        false
    }
}

fn panel_label(panel: Panel) -> &'static str {
    match panel {
        Panel::Processes => "Processes",
        Panel::Details => "Details",
        Panel::Resources => "Resources",
        Panel::Network => "Network",
        Panel::Disks => "Disks",
        Panel::More => "More",
    }
}

pub fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    let mut collectors = CollectorSet::new();
    for update in collectors.collect_all() {
        app.apply_update(update);
    }
    let mut last_refresh = Instant::now();
    let mut dirty = true;
    loop {
        if dirty {
            terminal.draw(|frame| ui::render(frame, app))?;
            dirty = false;
        }
        if last_refresh.elapsed() >= app.refresh {
            for update in collectors.collect_all() {
                app.apply_update(update);
            }
            last_refresh = Instant::now();
            dirty = true;
        }
        if app.clear_expired_notice() {
            dirty = true;
        }
        if event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key) => {
                    match app.handle_key(key) {
                        AppCommand::Quit => break,
                        AppCommand::Refresh => {
                            for update in collectors.collect_all() {
                                app.apply_update(update);
                            }
                            last_refresh = Instant::now();
                        }
                        AppCommand::ConfirmAction(action) => {
                            let pid = action.identity().pid;
                            match ProcessAction::execute(action) {
                                Ok(()) => app.show_notice(format!("Signal sent to PID {pid}.")),
                                Err(error) => app.show_notice(error.to_user_message()),
                            }
                            for update in collectors.collect_all() {
                                app.apply_update(update);
                            }
                            last_refresh = Instant::now();
                        }
                        _ => {}
                    }
                    dirty = true;
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse);
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        } else {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    Ok(())
}
