mod app;
mod config;
mod dialog;
mod feed;
mod fetch;
mod kline;
mod list;
mod quote;
mod render;
mod style;
mod text;
mod theme;

use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event};
use tenk::{DataClient, SourceKind};
use tokio::sync::mpsc;

use app::App;

pub async fn run(client: DataClient, source: SourceKind) -> Result<()> {
    let mut terminal = ratatui::init();

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut app = App::new(client, source, tx);
    app.on_startup();

    let tick = Duration::from_millis(100);
    let result = loop {
        terminal.draw(|f| render::render(f, &app))?;

        while let Ok(msg) = rx.try_recv() {
            app.apply_fetch(msg);
        }

        app.poll_if_due();

        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key) {
                    break Ok(());
                }
            }
        }

        if app.quit {
            break Ok(());
        }
    };

    app.save_on_exit();
    ratatui::restore();
    result
}
