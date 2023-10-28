use color_eyre::eyre::{eyre, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use deezer::DeezerClient;
use deezer_downloader::Downloader as DeezerDownloader;
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
    Queue(Id),
    Start(Id),
    Progress(Id, f32),
    Finish(Id),
    Error(Id),
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
                while let Ok(id) = _download_rx.recv() {
                    _progress_tx.send(DownloadProgress::Start(id)).unwrap();

                    let result = download_song(id).await;
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
                self.progress_tx.send(DownloadProgress::Queue(id)).unwrap();

                self.download_tx.send(id).expect("Channel should be open.");
            }
            DownloadRequest::Album(id) => {
                let _progress_tx = self.progress_tx.clone();
                let _download_tx = self.download_tx.clone();

                tokio::spawn(async move {
                    let client = DeezerClient::new();
                    let album = client.album(id).await.unwrap().unwrap();

                    for track in album.tracks {
                        _progress_tx
                            .send(DownloadProgress::Queue(track.id))
                            .unwrap();

                        _download_tx
                            .send(track.id)
                            .expect("Channel should be open.");
                    }
                });
            }
        };
    }
}

async fn download_song(id: Id) -> Result<()> {
    let downloader = DeezerDownloader::new().await.unwrap();
    let song = match downloader.download_song(id).await {
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
