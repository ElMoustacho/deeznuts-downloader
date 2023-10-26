mod app;
mod downloader;
mod tui;

use app::App;
use color_eyre::eyre::Result;
use ratatui::backend::CrosstermBackend as Backend;

// DEBUG: Test ids
static ALBUM_ID: u64 = 379962977;
static SONG_ID: u64 = 498469812;

pub type Frame<'a> = ratatui::Frame<'a, Backend<std::io::Stderr>>;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut app = App::default();
    app.run().await
}
