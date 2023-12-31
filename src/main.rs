mod app;
mod downloader;
mod log;
mod tui;

use app::App;
use color_eyre::eyre::Result;

pub type Frame<'a> = ratatui::Frame<'a>;

#[derive(Clone, Debug)]
pub enum Event {
    Error,
    Tick,
    Key(crossterm::event::KeyEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Tick,
    Quit,
    ToggleInputMode,
    Download,
    ScrollLogsUp,
    ScrollLogsDown,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut app = App::default();
    app.run().await
}
