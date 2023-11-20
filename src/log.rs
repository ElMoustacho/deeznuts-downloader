use crate::downloader::DownloadProgress;

#[derive(Debug)]
pub enum Log {
    Err(String),
    Ok(String),
}

pub fn get_log_msg(download_progress: &DownloadProgress) -> Option<Log> {
    match download_progress {
        DownloadProgress::Queue(_)
        | DownloadProgress::Start(_)
        | DownloadProgress::Progress(_, _) => None,
        DownloadProgress::Finish(id) => Some(Log::Ok(format!("Song with id {} downloaded.", id))),
        DownloadProgress::DownloadError(id) => Some(Log::Err(format!(
            "Error while downloading song with id {}.",
            id
        ))),
        DownloadProgress::SongNotFoundError(id) => {
            Some(Log::Err(format!("Song with id {} was not found.", id)))
        }
        DownloadProgress::AlbumNotFoundError(id) => {
            Some(Log::Err(format!("Album with id {} was not found.", id)))
        }
    }
}
