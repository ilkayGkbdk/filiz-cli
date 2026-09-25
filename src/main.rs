use std::io::{self, stdout, Write};
use std::time::Duration;

use crossterm::{
    cursor::{MoveTo, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use filiz::app;
use filiz::collectors::runtime::CollectorRuntime;
use filiz::ui::splash;
use ratatui::{backend::CrosstermBackend, Terminal};

const SPLASH: Duration = Duration::from_millis(650);

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, EnableMouseCapture) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

fn restore_terminal() {
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show);
    let _ = disable_raw_mode();
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Restore the terminal before the default hook prints, so the message stays visible.
/// Collector threads report their own panics as `Stopped`; the UI keeps running for those.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() == Some("main") {
            restore_terminal();
        }
        default(info);
    }));
}

fn main() -> anyhow::Result<()> {
    install_panic_hook();
    let _guard = TerminalGuard::enter()?;
    let (updates_tx, updates_rx) = std::sync::mpsc::channel();
    let runtime = CollectorRuntime::start(updates_tx);
    let logo = format!(
        "{}\n  for betül, with love ♡\n",
        include_str!("../assets/logo.ansi")
    );
    print!("{}", splash::for_raw_mode(&logo));
    stdout().flush()?;
    if event::poll(SPLASH)? {
        let _ = event::read()?;
    }
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = app::App::new(Duration::from_secs(2));
    app::run(&mut terminal, &mut app, runtime, updates_rx)
}
