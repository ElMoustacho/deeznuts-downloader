use std::fmt::Display;

use crate::downloader::{DownloadProgress, DownloadRequest, DownloadStatus, Downloader};
use crate::log::{get_log_msg, Log};
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
    input_mode: InputMode,
    logs: Vec<Log>,
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
            input_mode: InputMode::default(),
            logs: Vec::new(),
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
            if let Some(str) = get_log_msg(&progress) {
                self.logs.push(str);
            }

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
                    let pos = self
                        .queue
                        .iter()
                        .position(|x| x.song.id == id)
                        .expect("Track should be in queue.");
                    let mut elem = self.queue.remove(pos);
                    elem.status = DownloadStatus::Finished;
                }
                DownloadProgress::DownloadError(id) => {
                    let pos = self
                        .queue
                        .iter()
                        .position(|x| x.song.id == id)
                        .expect("Track should be in queue.");
                    let mut elem = self.queue.remove(pos);
                    elem.status = DownloadStatus::Error;
                }
                DownloadProgress::SongNotFoundError(_) => {}
                DownloadProgress::AlbumNotFoundError(_) => {}
            }
        }

        Ok(())
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }

    fn ui(&mut self, f: &mut Frame) -> Result<()> {
        let area = f.size();

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        let log_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(1), Constraint::Length(3)])
            .split(main_chunks[0]);

        let input_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Length(7), Constraint::Min(1)])
            .split(log_chunks[1]);

        self.render_logs(f, log_chunks[0]);

        f.render_widget(
            Paragraph::new(self.input_mode.to_string())
                .alignment(Alignment::Center)
                .block(Block::default().padding(Padding::uniform(1))),
            input_chunks[0],
        );
        self.render_input(f, input_chunks[1]);

        // Queue list
        self.render_queue_list(f, main_chunks[1]);

        Ok(())
    }

    fn render_logs(&mut self, f: &mut ratatui::prelude::Frame<'_>, rect: Rect) {
        f.render_widget(
            List::new(self.logs.iter().map(|x| format_log(x)).collect::<Vec<_>>())
                .block(Block::default().title("Logs").borders(Borders::all())),
            rect,
        );
    }

    fn render_input(&mut self, f: &mut Frame, rect: Rect) {
        f.render_widget(
            Paragraph::new(self.input.value()).block(
                Block::default()
                    .borders(Borders::all())
                    .padding(Padding::horizontal(1)),
            ),
            rect,
        );
        f.set_cursor(self.input.visual_cursor() as u16 + 2 + rect.x, rect.y + 1);
    }

    fn render_queue_list(&mut self, f: &mut Frame, rect: Rect) {
        f.render_widget(
            List::new(
                self.queue
                    .iter()
                    .map(|x| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("[{}]", x.status),
                                Style::default().fg(get_status_color(&x.status)).bold(),
                            ),
                            Span::styled(
                                format!(" {} ", x.song.artist.name),
                                Style::default().bold(),
                            ),
                            Span::raw(format!("- {}", x.song.title.clone())),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .borders(Borders::all())
                    .title("Download queue"),
            ),
            rect,
        );
    }
}

fn format_log(log: &Log) -> ListItem {
    let line = match log {
        Log::Ok(msg) => Line::from(vec![
            Span::styled("[Success] ", Style::default().fg(Color::LightGreen).bold()),
            Span::raw(msg),
        ]),
        Log::Err(err) => Line::from(vec![
            Span::styled("[Error] ", Style::default().fg(Color::Red).bold()),
            Span::raw(err),
        ]),
    };

    ListItem::new(line)
}

fn get_status_color(download_status: &DownloadStatus) -> Color {
    match download_status {
        DownloadStatus::Finished => Color::LightGreen,
        DownloadStatus::Downloading => Color::LightBlue,
        DownloadStatus::Error => Color::Red,
        DownloadStatus::Inactive => Color::Gray,
    }
}
