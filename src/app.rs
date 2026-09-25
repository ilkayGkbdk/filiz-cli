use std::io::Stdout;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, MouseEvent};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::actions::ProcessAction;
use crate::collectors::runtime::CollectorRuntime;
use crate::history::History;
use crate::input::action::{Action, Effect};
use crate::input::keymap::{self, Context};
use crate::input::list::ListState;
use crate::model::{
    filter_processes, sort_processes, ActionKind, AppMode, PendingAction, ProcessIdentity,
    ProcessInfo, SortMode,
};
use crate::state::{CollectorUpdate, InterfaceStats, SystemState};
use crate::ui::hit::{mouse_action, ClickMemory};
use crate::ui::state::{PanelId, PanelToggle, UiState, Workspace};
use crate::ui::{self, RenderOutput};

pub struct App {
    pub refresh: Duration,
    pub state: SystemState,
    pub history: History,
    pub mode: AppMode,
    pub sort: SortMode,
    pub filter: String,
    pub pending_action: Option<PendingAction>,
    pub notice: Option<String>,
    pub ui: UiState,
    pub last_render: RenderOutput,
    selected_identity: Option<ProcessIdentity>,
    notice_until: Option<Instant>,
    clicks: ClickMemory,
}

impl App {
    pub fn new(refresh: Duration) -> Self {
        Self {
            refresh,
            state: SystemState::default(),
            history: History::new(60),
            mode: AppMode::Dashboard,
            sort: SortMode::Cpu,
            filter: String::new(),
            pending_action: None,
            notice: None,
            ui: UiState::default(),
            last_render: RenderOutput::default(),
            selected_identity: None,
            notice_until: None,
            clicks: ClickMemory::default(),
        }
    }

    pub fn apply_update(&mut self, update: CollectorUpdate) {
        let is_system = matches!(update, CollectorUpdate::System(_));
        self.state.apply(update);
        if is_system {
            self.history.record(&self.state);
        }
        self.reconcile_selection();
        for panel in [PanelId::Interfaces, PanelId::Traffic, PanelId::Disks] {
            let (len, viewport) = (self.list_len(panel), self.viewport(panel));
            self.ui.list_mut(panel).clamp(len, viewport);
        }
    }

    /// Apply every queued collector update without blocking. Returns true if anything changed.
    pub fn pump(&mut self, updates: &Receiver<CollectorUpdate>) -> bool {
        let mut changed = false;
        while let Ok(update) = updates.try_recv() {
            self.apply_update(update);
            changed = true;
        }
        changed
    }

    pub fn visible_processes(&self) -> Vec<ProcessInfo> {
        let mut processes = filter_processes(&self.state.processes, &self.filter);
        sort_processes(&mut processes, self.sort);
        processes
    }

    pub fn selected_process(&self) -> Option<ProcessInfo> {
        self.visible_processes()
            .get(self.ui.list(PanelId::Processes).selected)
            .cloned()
    }

    pub fn selected_interface(&self) -> Option<&InterfaceStats> {
        self.state
            .interfaces
            .get(self.ui.list(PanelId::Interfaces).selected)
    }

    /// Processes with measured traffic, busiest first.
    pub fn traffic_rows(&self) -> Vec<&ProcessInfo> {
        let mut rows: Vec<&ProcessInfo> = self
            .state
            .processes
            .iter()
            .filter(|process| process.traffic.is_some())
            .collect();
        let total = |process: &ProcessInfo| process.traffic.map_or(0.0, |t| t.rx + t.tx);
        rows.sort_by(|a, b| total(b).total_cmp(&total(a)));
        rows
    }

    pub fn list_len(&self, panel: PanelId) -> usize {
        match panel {
            PanelId::Processes => self.visible_processes().len(),
            PanelId::Interfaces => self.state.interfaces.len(),
            PanelId::Traffic => self.traffic_rows().len(),
            PanelId::Disks => self.state.visible_disks().len(),
            PanelId::Resources | PanelId::Details | PanelId::Settings => 0,
        }
    }

    pub fn viewport(&self, panel: PanelId) -> usize {
        self.last_render
            .viewports
            .get(&panel)
            .copied()
            .unwrap_or(1)
            .max(1)
    }

    pub fn contexts(&self) -> Vec<Context> {
        match self.mode {
            AppMode::ConfirmingAction => vec![Context::Confirm],
            AppMode::Filtering => vec![Context::Filter],
            AppMode::ProcessDetail => vec![Context::Detail, Context::Global],
            AppMode::Dashboard => {
                let mut stack = Vec::new();
                if self.ui.menu_open {
                    stack.push(Context::Menu);
                }
                stack.push(Context::Panel(self.ui.focus));
                stack.push(Context::Workspace(self.ui.workspace));
                stack.push(Context::Global);
                stack
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Vec<Effect> {
        match keymap::resolve(&self.contexts(), &key) {
            Some(action) => self.update(action),
            None => Vec::new(),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Vec<Effect> {
        if self.mode == AppMode::Filtering {
            return Vec::new();
        }
        match mouse_action(
            &self.last_render.hits,
            &mouse,
            self.ui.focus,
            &mut self.clicks,
            Instant::now(),
        ) {
            Some(action) => self.update(action),
            None => Vec::new(),
        }
    }

    pub fn update(&mut self, action: Action) -> Vec<Effect> {
        let focus = self.ui.focus;
        match action {
            Action::Quit => return vec![Effect::Quit],
            Action::Refresh => return vec![Effect::Refresh],
            Action::GoWorkspace(workspace) => self.go_workspace(workspace),
            Action::NextWorkspace => self.go_workspace(self.ui.workspace.step(1)),
            Action::PrevWorkspace => self.go_workspace(self.ui.workspace.step(-1)),
            Action::FocusNext => self.ui.focus_step(1),
            Action::FocusPrev => self.ui.focus_step(-1),
            Action::Focus(panel) => {
                if self.ui.visible_panels().contains(&panel) {
                    self.ui.focus = panel;
                }
            }
            Action::MoveUp => self.move_list(focus, -1),
            Action::MoveDown => self.move_list(focus, 1),
            Action::PageUp => self.move_list(focus, -(self.viewport(focus) as isize)),
            Action::PageDown => self.move_list(focus, self.viewport(focus) as isize),
            Action::Home => self.move_list(focus, isize::MIN / 2),
            Action::End => self.move_list(focus, isize::MAX / 2),
            Action::Scroll(panel, delta) => self.move_list(panel, isize::from(delta)),
            Action::Select(panel, index) => {
                self.ui.focus = panel;
                let current = self.ui.list(panel).selected as isize;
                self.move_list(panel, index as isize - current);
            }
            Action::Open => {
                if focus == PanelId::Processes && self.selected_process().is_some() {
                    self.mode = AppMode::ProcessDetail;
                }
            }
            Action::Back => self.back(),
            Action::StartFilter => self.mode = AppMode::Filtering,
            Action::FilterInput(character) => {
                self.filter.push(character);
                self.select_first();
            }
            Action::FilterBackspace => {
                self.filter.pop();
                self.select_first();
            }
            Action::FilterSubmit => self.mode = AppMode::Dashboard,
            Action::CycleSort => {
                self.sort = match self.sort {
                    SortMode::Cpu => SortMode::Memory,
                    SortMode::Memory => SortMode::Cpu,
                };
                self.reconcile_selection();
            }
            Action::Terminate => self.begin_action(ActionKind::Terminate),
            Action::Kill => self.begin_action(ActionKind::Kill),
            Action::Confirm => {
                self.mode = AppMode::Dashboard;
                if let Some(pending) = self.pending_action.take() {
                    return vec![Effect::SendSignal(pending.confirm())];
                }
            }
            Action::Cancel => {
                self.mode = AppMode::Dashboard;
                if let Some(pending) = self.pending_action.take() {
                    let _ = pending.cancel();
                }
            }
            Action::ToggleMenu => self.ui.menu_open = !self.ui.menu_open,
            Action::CycleTheme => {
                self.ui.theme = self.ui.theme.next();
                self.show_notice(format!("Theme: {}", self.ui.theme.label()));
            }
            Action::CycleDensity => {
                self.ui.density = self.ui.density.cycle();
                self.show_notice(format!("Layout: {:?}", self.ui.density));
            }
            Action::TogglePanel => {
                let message = match self.ui.toggle_panel() {
                    PanelToggle::Hidden(panel) => {
                        format!("{} panel hidden. Press H to restore.", panel.label())
                    }
                    PanelToggle::Restored => "All panels visible.".to_owned(),
                    PanelToggle::LastPanel => "The last visible panel cannot be hidden.".to_owned(),
                };
                self.show_notice(message);
            }
        }
        Vec::new()
    }

    fn go_workspace(&mut self, workspace: Workspace) {
        self.ui.set_workspace(workspace);
        self.show_notice(format!("Workspace: {}", workspace.label()));
    }

    fn back(&mut self) {
        if self.mode == AppMode::ProcessDetail {
            self.mode = AppMode::Dashboard;
        } else if self.ui.menu_open {
            self.ui.menu_open = false;
        } else if !self.filter.is_empty() {
            self.filter.clear();
            self.select_first();
        }
    }

    fn move_list(&mut self, panel: PanelId, delta: isize) {
        let (len, viewport) = (self.list_len(panel), self.viewport(panel));
        self.ui.list_mut(panel).move_by(delta, len, viewport);
        if panel == PanelId::Processes {
            self.selected_identity = self.selected_process().map(|process| process.identity);
        }
    }

    fn begin_action(&mut self, kind: ActionKind) {
        let Some(process) = self.selected_process() else {
            return;
        };
        if process.identity.pid == std::process::id() {
            self.show_notice("Filiz cannot send a signal to itself.");
            return;
        }
        self.pending_action = Some(PendingAction::new(process.identity, kind));
        self.mode = AppMode::ConfirmingAction;
    }

    fn select_first(&mut self) {
        *self.ui.list_mut(PanelId::Processes) = ListState::default();
        self.selected_identity = self.visible_processes().first().map(|p| p.identity);
    }

    fn reconcile_selection(&mut self) {
        let processes = self.visible_processes();
        let previous = self.selected_identity;
        let index = previous
            .and_then(|identity| processes.iter().position(|p| p.identity == identity))
            .or_else(|| {
                processes
                    .iter()
                    .position(|p| previous.is_none_or(|identity| p.identity.pid != identity.pid))
            })
            .unwrap_or(0);
        let viewport = self.viewport(PanelId::Processes);
        let list = self.ui.list_mut(PanelId::Processes);
        list.selected = index;
        list.clamp(processes.len(), viewport);
        self.selected_identity = processes.get(list.selected).map(|p| p.identity);
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

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    runtime: CollectorRuntime,
    updates: Receiver<CollectorUpdate>,
) -> Result<()> {
    // `updates` must be dropped before `runtime`: dropping `runtime` joins the
    // collector threads, and they can block sending on `updates` until its
    // receiver is gone. Params drop in reverse declaration order (`updates`
    // then `runtime`) on every return path here, so this ordering holds as-is.
    const INPUT_WAIT: Duration = Duration::from_millis(50);
    let mut dirty = true;
    loop {
        if app.pump(&updates) {
            dirty = true;
        }
        if app.clear_expired_notice() {
            dirty = true;
        }
        if dirty {
            let mut output = RenderOutput::default();
            terminal.draw(|frame| output = ui::render(frame, app))?;
            app.last_render = output;
            dirty = false;
        }
        if !event::poll(INPUT_WAIT)? {
            continue;
        }
        let effects = match event::read()? {
            Event::Key(key) => app.handle_key(key),
            Event::Mouse(mouse) => app.handle_mouse(mouse),
            Event::Resize(_, _) => Vec::new(),
            _ => continue,
        };
        dirty = true;
        for effect in effects {
            match effect {
                Effect::Quit => return Ok(()),
                Effect::Refresh => runtime.refresh(),
                Effect::SendSignal(action) => {
                    let pid = action.identity().pid;
                    match ProcessAction::execute(action) {
                        Ok(()) => app.show_notice(format!("Signal sent to PID {pid}.")),
                        Err(error) => app.show_notice(error.to_user_message()),
                    }
                    runtime.refresh();
                }
            }
        }
    }
}
