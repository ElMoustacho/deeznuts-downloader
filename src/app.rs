use crate::downloader::{DownloadRequest, Downloader};
use crate::{tui::Tui, Action, Event, Frame};
use color_eyre::eyre::{eyre, Result};
use ratatui::{prelude::*, widgets::*};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    input: Input,
    downloader: Downloader,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            input: Input::default(),
            downloader: Downloader::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
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

    fn handle_event(&mut self, event: Event) -> Result<Action> {
        let msg = match event {
            Event::Key(key) => match key.code {
                crossterm::event::KeyCode::Esc => Action::Quit,
                crossterm::event::KeyCode::Enter => Action::DownloadSong,
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    Action::Tick
                }
            },
            _ => Action::Tick,
        };
        Ok(msg)
    }

    fn update(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Tick => {}
            Action::Quit => self.quit(),
            Action::DownloadSong => {
                if let Ok(id) = self.input.value().parse::<u64>() {
                    self.downloader.request_download(DownloadRequest::Song(id));
                }
            }
            Action::DownloadAlbum => {
                if let Ok(id) = self.input.value().parse::<u64>() {
                    self.downloader.request_download(DownloadRequest::Album(id));
                }
            }
        }
        Ok(())
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }

    fn ui(&mut self, f: &mut Frame) -> Result<()> {
        let area = f.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(vec![Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        f.render_widget(
            Paragraph::new(self.input.value()).block(Block::default().borders(Borders::all())),
            chunks[1],
        );
        f.set_cursor(self.input.visual_cursor() as u16 + 2, chunks[1].y + 1);

        Ok(())
    }
}
