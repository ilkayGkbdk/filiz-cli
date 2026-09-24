use std::io::{self, stdout, Write};
use std::time::Duration;

use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use filiz::app;
use ratatui::{backend::CrosstermBackend, Terminal};

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn main() -> anyhow::Result<()> {
    let _guard = TerminalGuard::enter()?;
    print!(
        "{}\n  for betül, with love ♡\n",
        include_str!("../assets/logo.ansi")
    );
    stdout().flush()?;
    std::thread::sleep(Duration::from_millis(650));
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = app::App::new(Duration::from_secs(2));
    app::run(&mut terminal, &mut app)
}
