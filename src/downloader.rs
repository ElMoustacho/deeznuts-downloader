use std::fmt::Display;

use color_eyre::eyre::{eyre, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use deezer::{models::Track, DeezerClient};
use deezer_downloader::{
    song::{Album, Artist},
    Downloader as DeezerDownloader, Song, SongMetadata,
};
use directories::UserDirs;
use futures::{future::join_all, TryFutureExt};

static DOWNLOAD_THREADS: u64 = 4;

type Id = u64;

#[derive(Debug)]
pub enum DownloadRequest {
    Album(Id),
    Song(Id),
}

#[derive(Debug)]
pub enum DownloadProgress {
    Queue(Track),
    Start(Track),
    Finish(Track),
    DownloadError(Track),
    SongNotFoundError(Id),
    AlbumNotFoundError(Id),
}

#[derive(Debug)]
pub enum DownloadStatus {
    Downloading,
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
    download_tx: Sender<Track>,
}

impl Downloader {
    pub fn new() -> Self {
        let (download_tx, download_rx) = unbounded::<Track>();
        let (progress_tx, progress_rx) = unbounded();

        for _ in 0..DOWNLOAD_THREADS {
            let _download_rx = download_rx.clone();
            let _progress_tx = progress_tx.clone();

            tokio::spawn(async move {
                let downloader = DeezerDownloader::new().await.unwrap();
                while let Ok(track) = _download_rx.recv() {
                    _progress_tx
                        .send(DownloadProgress::Start(track.clone()))
                        .unwrap();

                    let result = download_song_from_track(track.clone(), &downloader).await;
                    let progress = match result {
                        Ok(_) => DownloadProgress::Finish(track),
                        Err(_) => DownloadProgress::DownloadError(track),
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

                tokio::spawn(download_song(id, _progress_tx, _download_tx));
            }
            DownloadRequest::Album(id) => {
                let _progress_tx = self.progress_tx.clone();
                let _download_tx = self.download_tx.clone();

                tokio::spawn(download_album(id, _progress_tx, _download_tx));
            }
        };
    }
}

async fn download_song(id: u64, progress_tx: Sender<DownloadProgress>, download_tx: Sender<Track>) {
    let client = DeezerClient::new();
    let maybe_track = client.track(id).await;

    // Check if the song was found AND is readable
    match maybe_track {
        Ok(Some(track)) if track.readable => {
            progress_tx
                .send(DownloadProgress::Queue(track.clone()))
                .expect("Channel should be open.");
            download_tx.send(track).expect("Channel should be open.");
        }
        _ => {
            progress_tx
                .send(DownloadProgress::SongNotFoundError(id))
                .expect("Channel should be open.");
        }
    }
}

async fn download_album(
    id: u64,
    progress_tx: Sender<DownloadProgress>,
    download_tx: Sender<Track>,
) {
    let client = DeezerClient::new();
    let maybe_album = client.album(id).await;

    if let Ok(Some(album)) = maybe_album {
        let mut futures = Vec::new();

        for (index, album_track) in album.tracks.iter().enumerate() {
            // Add current loop index to Track; we're doing this istead of using
            // the default value because it starts over when an album has multiple CDs
            let track = album_track.get_full().and_then(move |mut x| async move {
                x.track_position_in_album = (index + 1) as u64;
                Ok(x)
            });

            futures.push(async {
                let track = track.await.expect("Track should always be available.");

                progress_tx
                    .send(DownloadProgress::Queue(track.clone()))
                    .expect("Channel should be open.");
                download_tx.send(track).expect("Channel should be open.");
            });
        }

        join_all(futures).await;
    } else {
        progress_tx
            .send(DownloadProgress::AlbumNotFoundError(id))
            .expect("Channel should be open.");
    }
}

async fn download_song_from_track(track: Track, downloader: &DeezerDownloader) -> Result<()> {
    let id = track.id;
    let mut song = match Song::download_from_metadata(metadata_from_track(&track), downloader).await
    {
        Ok(it) => it,
        Err(_) => return Err(eyre!(format!("Song with id {} not found.", id))),
    };

    song.tag.set_track(track.track_position_in_album as u32);

    write_song_to_file(song)?;

    Ok(())
}

/// Write a [Song] to the download directory.
///
/// TODO: Allow the target directory to be given.
fn write_song_to_file(song: Song) -> Result<()> {
    let Some(user_dirs) = UserDirs::new() else {
        return Ok(());
    };

    if let Some(download_dirs) = user_dirs.download_dir() {
        let song_title = format!(
            "{} - {}.mp3",
            song.tag.artist().unwrap_or_default(),
            song.tag.title().unwrap_or_default()
        );
        let song_title = replace_illegal_characters(&song_title);

        song.write_to_file(download_dirs.join(song_title))
            .map_err(|_| eyre!("An error occured while writing the file."))?;
    }

    Ok(())
}

/// Replaces illegal characters for a Windows file.
fn replace_illegal_characters(str: &str) -> String {
    static ILLEGAL_CHARACTERS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    str.chars()
        .filter(|char| !ILLEGAL_CHARACTERS.contains(char))
        .collect()
}

fn metadata_from_track(track: &Track) -> SongMetadata {
    SongMetadata {
        id: track.id,
        title: track.title.clone(),
        artist: Artist {
            id: track.artist.id.clone(),
            name: track.artist.name.clone(),
        },
        album: Album {
            id: track.album.id.clone(),
            title: track.album.title.clone(),
            cover_small: track.album.cover_small.clone(),
            cover_medium: track.album.cover_medium.clone(),
            cover_big: track.album.cover_big.clone(),
        },
        release_date: Some(track.release_date.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_invalid_chars() {
        let file_name = "AC/DC - Thunderstruck.mp3";

        assert_eq!(
            "ACDC - Thunderstruck.mp3",
            replace_illegal_characters(file_name)
        );

        let file_name = "<>:\"/\\|?* - Test.mp3";

        assert_eq!(" - Test.mp3", replace_illegal_characters(file_name));
    }
}
