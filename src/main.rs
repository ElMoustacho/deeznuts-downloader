mod downloader;
mod tui;

use color_eyre::eyre::{eyre, Result};
use ratatui::{backend::CrosstermBackend as Backend, prelude::*, widgets::*};
use tui::Tui;

// DEBUG: Test ids
static ALBUM_ID: u64 = 379962977;
static SONG_ID: u64 = 498469812;

pub type Frame<'a> = ratatui::Frame<'a, Backend<std::io::Stderr>>;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::default();
    app.run().await
}

#[derive(Clone, Debug)]
pub enum Event {
    Error,
    Tick,
    Key(crossterm::event::KeyEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Message {
    Tick,
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
struct App {
    should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    fn new() -> Self {
        Self { should_quit: false }
    }

    async fn run(&mut self) -> Result<()> {
        let mut tui = Tui::new()?;
        tui.enter()?;
        while !self.should_quit {
            tui.draw(|f| self.ui(f).expect("Unexpected error during drawing"))?;
            let event = tui.next().await.ok_or(eyre!("Unable to get event"))?; // blocks until next event
            let message = self.handle_event(event)?;
            self.update(message)?;
        }
        tui.exit()?;
        Ok(())
    }

    fn handle_event(&self, event: Event) -> Result<Message> {
        let msg = match event {
            Event::Key(key) => match key.code {
                crossterm::event::KeyCode::Char('q') => Message::Quit,
                _ => Message::Tick,
            },
            _ => Message::Tick,
        };
        Ok(msg)
    }

    fn update(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Tick => {}
            Message::Quit => self.quit(),
        }
        Ok(())
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }

    fn ui(&mut self, f: &mut Frame) -> Result<()> {
        let area = f.size();
        f.render_widget(Paragraph::new("Coucou les gens!").blue(), area);

        Ok(())
    }
}
