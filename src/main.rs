use color_eyre::eyre::Result;

mod anim;
mod api;
mod app;
mod event;
mod models;
mod terminal;
mod ui;
mod widgets;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = terminal::init()?;
    let mut app = app::App::new();
    let mut event_handler = event::EventHandler::new();

    let result = run_app(&mut terminal, &mut app, &mut event_handler).await;

    terminal::restore()?;

    result
}

async fn run_app(
    terminal: &mut terminal::Terminal,
    app: &mut app::App,
    event_handler: &mut event::EventHandler,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        app.process_pending_operations().await?;

        if let Some(event) = event_handler.next()?
            && !app.handle_event(event).await?
        {
            return Ok(());
        }
    }
}
