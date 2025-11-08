use {
    color_eyre::eyre::Result,
    crossterm::{
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    ratatui::{Terminal as RatatuiTerminal, backend::CrosstermBackend},
    std::io::{self, Stdout},
};

pub type Terminal = RatatuiTerminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Terminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
