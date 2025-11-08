use {
    color_eyre::eyre::Result,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    std::time::Duration,
};

#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Key(KeyCode),
    Tick,
}

pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            tick_rate: Duration::from_millis(100),
        }
    }

    pub fn next(&self) -> Result<Option<AppEvent>> {
        if event::poll(self.tick_rate)?
            && let Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) = event::read()?
        {
            return Ok(Some(AppEvent::Key(code)));
        }

        Ok(Some(AppEvent::Tick))
    }
}
