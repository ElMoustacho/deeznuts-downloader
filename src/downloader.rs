use std::fmt::Display;

use color_eyre::eyre::{eyre, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use deezer::{models::Track, DeezerClient};
use deezer_downloader::{Downloader as DeezerDownloader, Song};
use directories::UserDirs;

static DOWNLOAD_THREADS: Id = 4;

type Id = u64;

#[derive(Debug)]
pub enum DownloadRequest {
    Album(Id),
    Song(Id),
}

#[derive(Debug)]
pub enum DownloadProgress {
    Queue(Track),
    Start(Id),
    Progress(Id, f32),
    Finish(Id),
    Error(Id),
}

#[derive(Debug)]
pub enum DownloadStatus {
    Finished,
    Downloading,
    Error,
    Inactive,
}

impl Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct Downloader {
    pub progress_rx: Receiver<DownloadProgress>,
    progress_tx: Sender<DownloadProgress>,
    download_tx: Sender<Id>,
}

impl Downloader {
    pub fn new() -> Self {
        let (download_tx, download_rx) = unbounded();
        let (progress_tx, progress_rx) = unbounded();

        for _ in 0..DOWNLOAD_THREADS {
            let _download_rx = download_rx.clone();
            let _progress_tx = progress_tx.clone();

            tokio::spawn(async move {
                let downloader = DeezerDownloader::new().await.unwrap();
                while let Ok(id) = _download_rx.recv() {
                    _progress_tx.send(DownloadProgress::Start(id)).unwrap();

                    let result = download_song(id, &downloader).await;
                    let progress = match result {
                        Ok(_) => DownloadProgress::Finish(id),
                        Err(_) => DownloadProgress::Error(id),
                    };

                    _progress_tx.send(progress).unwrap();
                }
            });
        }

        Downloader {
            download_tx,
            progress_tx,
            progress_rx,
        }
    }

    pub fn request_download(&self, request: DownloadRequest) {
        match request {
            DownloadRequest::Song(id) => {
                let _progress_tx = self.progress_tx.clone();
                let _download_tx = self.download_tx.clone();

                tokio::spawn(async move {
                    let client = DeezerClient::new();
                    let maybe_track = client.track(id).await;

                    if let Ok(Some(track)) = maybe_track {
                        _progress_tx
                            .send(DownloadProgress::Queue(track))
                            .expect("Channel should be open.");
                        _download_tx.send(id).expect("Channel should be open.");
                    } else {
                        // TODO: Display error message indicating the song hasn't been found
                    }
                });
            }
            DownloadRequest::Album(id) => {
                let _progress_tx = self.progress_tx.clone();
                let _download_tx = self.download_tx.clone();

                tokio::spawn(async move {
                    let client = DeezerClient::new();
                    let maybe_album = client.album(id).await;

                    if let Ok(Some(album)) = maybe_album {
                        for album_track in album.tracks {
                            let track = album_track
                                .get_full()
                                .await
                                .expect("Track should always be available.");
                            let id = track.id;

                            _progress_tx
                                .send(DownloadProgress::Queue(track))
                                .expect("Channel should be open.");
                            _download_tx.send(id).expect("Channel should be open.");
                        }
                    } else {
                        // TODO: Display error message indicating the album hasn't been found
                    }
                });
            }
        };
    }
}

async fn download_song(id: Id, downloader: &DeezerDownloader) -> Result<()> {
    let song = match Song::download(id, downloader).await {
        Ok(it) => it,
        Err(_) => return Err(eyre!(format!("Song with id {} not found.", id))),
    };

    if let Some(user_dirs) = UserDirs::new() {
        if let Some(download_dirs) = user_dirs.download_dir() {
            let song_title = format!(
                "./{} - {}.mp3",
                song.tag.artist().unwrap_or_default(),
                song.tag.title().unwrap_or_default()
            );

            song.write_to_file(download_dirs.join(song_title))
                .expect("An error occured while writing the file.");
        }
    }

    Ok(())
}
