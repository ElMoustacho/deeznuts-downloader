use crate::downloader::DownloadProgress;

#[derive(Debug)]
pub enum LogEntry {
    Error(String),
    Success(String),
}

pub fn get_log_from_progress(download_progress: &DownloadProgress) -> Option<LogEntry> {
    match download_progress {
        DownloadProgress::Queue(_)
        | DownloadProgress::Start(_)
        | DownloadProgress::Progress(_, _) => None,
        DownloadProgress::Finish(id) => Some(LogEntry::Success(format!(
            "Song with id {} downloaded.",
            id
        ))),
        DownloadProgress::DownloadError(id) => Some(LogEntry::Error(format!(
            "Error while downloading song with id {}.",
            id
        ))),
        DownloadProgress::SongNotFoundError(id) => Some(LogEntry::Error(format!(
            "Song with id {} was not found.",
            id
        ))),
        DownloadProgress::AlbumNotFoundError(id) => Some(LogEntry::Error(format!(
            "Album with id {} was not found.",
            id
        ))),
    }
}
