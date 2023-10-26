use color_eyre::eyre::{eyre, Result};
use deezer::DeezerClient;
use deezer_downloader::Downloader as DeezerDownloader;
use directories::UserDirs;
use futures::future::join_all;

type Id = u64;

pub enum DownloadRequest {
    Album(Id),
    Song(Id),
}

#[derive(Debug)]
pub struct Downloader {}

impl Downloader {
    pub fn new() -> Self {
        Downloader {}
    }

    pub fn request_download(&self, request: DownloadRequest) {
        match request {
            DownloadRequest::Album(id) => tokio::spawn(download_album(id)),
            DownloadRequest::Song(id) => tokio::spawn(download_song(id)),
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

pub async fn download_album(id: Id) -> Result<()> {
    let mut futures = Vec::new();
    let client = DeezerClient::new();

    let album = client
        .album(id)
        .await
        .transpose()
        .ok_or(eyre!("Album not found."))??;

    for song in album.tracks {
        futures.push(download_song(song.id));
    }

    join_all(futures).await;

    Ok(())
}
