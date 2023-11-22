use crate::downloader::DownloadProgress;

#[derive(Debug)]
pub enum LogEntry {
    Error(String),
    Success(String),
}

pub fn get_log_from_progress(download_progress: &DownloadProgress) -> Option<LogEntry> {
    match download_progress {
        DownloadProgress::Queue(_) | DownloadProgress::Start(_) => None,
        DownloadProgress::Finish(track) => Some(LogEntry::Success(format!(
            "{} - {} downloaded",
            track.artist.name, track.title
        ))),
        DownloadProgress::DownloadError(track) => Some(LogEntry::Error(format!(
            "Error while downloading {} - {}",
            track.artist.name, track.title
        ))),
        DownloadProgress::SongNotFoundError(id) => Some(LogEntry::Error(format!(
            "Song with id {} was not found",
            id
        ))),
        DownloadProgress::AlbumNotFoundError(id) => Some(LogEntry::Error(format!(
            "Album with id {} was not found",
            id
        ))),
    }
}
