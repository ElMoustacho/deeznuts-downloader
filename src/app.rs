use std::fmt::Display;

use crate::downloader::{DownloadProgress, DownloadRequest, DownloadStatus, Downloader};
use crate::{tui::Tui, Action, Event, Frame};
use color_eyre::eyre::{eyre, Result};
use deezer::models::Track;
use ratatui::{prelude::*, widgets::*};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

#[derive(Debug, Default)]
enum InputMode {
    #[default]
    Song,
    Album,
}

impl Display for InputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
struct QueueItem {
    pub song: Track,
    pub status: DownloadStatus,
}

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    input: Input,
    downloader: Downloader,
    queue: Vec<QueueItem>,
    finished_queue: Vec<QueueItem>,
    input_mode: InputMode,
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
            queue: Vec::new(),
            finished_queue: Vec::new(),
            input_mode: InputMode::default(),
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
                crossterm::event::KeyCode::Enter => Action::Download,
                crossterm::event::KeyCode::Tab => Action::ToggleInputMode,
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
            Action::ToggleInputMode => {
                self.input_mode = match self.input_mode {
                    InputMode::Song => InputMode::Album,
                    InputMode::Album => InputMode::Song,
                }
            }
            Action::Download => {
                let request = match self.input_mode {
                    InputMode::Song => DownloadRequest::Song,
                    InputMode::Album => DownloadRequest::Album,
                };

                if let Ok(id) = self.input.value().parse::<u64>() {
                    self.input.reset();
                    self.downloader.request_download(request(id));
                }
            }
        }

        while let Ok(progress) = self.downloader.progress_rx.try_recv() {
            match progress {
                DownloadProgress::Queue(track) => self.queue.push(QueueItem {
                    song: track,
                    status: DownloadStatus::Inactive,
                }),
                DownloadProgress::Start(id) => {
                    for item in self.queue.iter_mut() {
                        if item.song.id == id {
                            item.status = DownloadStatus::Downloading
                        }
                    }
                }
                DownloadProgress::Progress(_, _) => {}
                DownloadProgress::Finish(id) => {
                    let pos = self.queue.iter().position(|x| x.song.id == id).unwrap();
                    let mut elem = self.queue.remove(pos);
                    elem.status = DownloadStatus::Finished;

                    self.finished_queue.push(elem)
                }
                DownloadProgress::Error(id) => {
                    let pos = self.queue.iter().position(|x| x.song.id == id).unwrap();
                    let mut elem = self.queue.remove(pos);
                    elem.status = DownloadStatus::Error;

                    self.finished_queue.push(elem)
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

        let input_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(7), Constraint::Min(1)])
            .split(chunks[1]);

        // Search bar
        f.render_widget(
            Paragraph::new(self.input_mode.to_string())
                .alignment(Alignment::Center)
                .block(Block::default().padding(Padding::uniform(1))),
            input_chunks[0],
        );

        f.render_widget(
            Paragraph::new(self.input.value()).block(
                Block::default()
                    .borders(Borders::all())
                    .padding(Padding::horizontal(1)),
            ),
            input_chunks[1],
        );
        f.set_cursor(
            self.input.visual_cursor() as u16 + 2 + input_chunks[1].x,
            input_chunks[1].y + 1,
        );

        let chunks2 = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        // Queue list
        f.render_widget(
            List::new(
                self.queue
                    .iter()
                    .map(|x| ListItem::new(format!("{} [{}]", x.song.title, x.status)))
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .borders(Borders::all())
                    .title("Download queue"),
            ),
            chunks2[0],
        );

        // Finished list
        f.render_widget(
            List::new(
                self.finished_queue
                    .iter()
                    .map(|x| ListItem::new(format!("{} [{}]", x.song.title, x.status)))
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .borders(Borders::all())
                    .title("Finished downloading"),
            ),
            chunks2[1],
        );

        Ok(())
    }
}
