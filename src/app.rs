use crate::downloader::{DownloadProgress, DownloadRequest, DownloadStatus, Downloader};
use crate::{tui::Tui, Action, Event, Frame};
use color_eyre::eyre::{eyre, Result};
use ratatui::{prelude::*, widgets::*};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

#[derive(Debug)]
struct QueueItem {
    pub id: u64,
    pub status: DownloadStatus,
}

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    input: Input,
    downloader: Downloader,
    queue: Vec<QueueItem>,
    finished_queue: Vec<QueueItem>,
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
                crossterm::event::KeyCode::Tab => Action::DownloadAlbum,
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
                    self.input.reset();
                    self.downloader.request_download(DownloadRequest::Song(id));
                }
            }
            Action::DownloadAlbum => {
                if let Ok(id) = self.input.value().parse::<u64>() {
                    self.input.reset();
                    self.downloader.request_download(DownloadRequest::Album(id));
                }
            }
        }

        while let Ok(progress) = self.downloader.progress_rx.try_recv() {
            match progress {
                DownloadProgress::Queue(id) => self.queue.push(QueueItem {
                    id,
                    status: DownloadStatus::Inactive,
                }),
                DownloadProgress::Start(id) => {
                    for item in self.queue.iter_mut() {
                        if item.id == id {
                            item.status = DownloadStatus::Downloading
                        }
                    }
                }
                DownloadProgress::Progress(_, _) => {}
                DownloadProgress::Finish(id) => {
                    let pos = self.queue.iter().position(|x| x.id == id).unwrap();
                    let mut elem = self.queue.remove(pos);
                    elem.status = DownloadStatus::Finished;

                    self.finished_queue.push(elem)
                }
                DownloadProgress::Error(id) => {
                    let pos = self.queue.iter().position(|x| x.id == id).unwrap();
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

        // Search bar
        f.render_widget(
            Paragraph::new(self.input.value()).block(Block::default().borders(Borders::all())),
            chunks[1],
        );
        f.set_cursor(self.input.visual_cursor() as u16 + 2, chunks[1].y + 1);

        let chunks2 = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        // Queue list
        f.render_widget(
            List::new(
                self.queue
                    .iter()
                    .map(|x| ListItem::new(format!("{} [{}]", x.id, x.status)))
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
                    .map(|x| ListItem::new(format!("{} [{}]", x.id, x.status)))
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
