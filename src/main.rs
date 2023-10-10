#![allow(dead_code)]
#![allow(unused_variables)]

use deezer::DeezerClient;
use deezer_downloader::downloader::Downloader;
use directories::UserDirs;
use futures::future::join_all;

// DEBUG: Test ids
static ALBUM_ID: u64 = 379962977;
static SONG_ID: u64 = 498469812;

async fn download_song(id: u64, counter: Option<u32>) {
    let downloader = Downloader::new().await.unwrap();
    let mut song = downloader.download_song(id).await.unwrap();
    let client = DeezerClient::new();

    if let Some(user_dirs) = UserDirs::new() {
        if let Some(download_dirs) = user_dirs.download_dir() {
            let song_title = format!(
                "./{} - {}.mp3",
                song.tag.artist().unwrap_or_default(),
                song.tag.title().unwrap_or_default()
            );

            if let Some(counter) = counter {
                song.tag.set_track(counter);
            }

            song.write_to_file(download_dirs.join(song_title))
                .expect("An error occured while writing the file.");
        }
    }
}

async fn download_album(id: u64) {
    let mut futures = Vec::new();
    let client = DeezerClient::new();

    let mut counter = 1;
    let album = client.album(id).await.unwrap().unwrap();
    for song in album.tracks {
        futures.push(download_song(song.id, Some(counter)));
        counter += 1;
    }

    join_all(futures).await;
}

#[tokio::main]
async fn main() {
    print!("Enter the ID of the album you want to download: ");

    let mut input_string = String::new();

    loop {
        input_string.clear();
        std::io::stdin().read_line(&mut input_string).unwrap();

        if let Ok(id) = input_string.trim().parse::<u64>() {
            download_album(id).await;

            break;
        }
    }

    println!("Download successful!");
}
