use std::io::Write;

use bytes::BufMut;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use warp::multipart::{FormData, Part};

use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct Account {
    pub id: u64,
    pub thumb: String,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Server {
    pub title: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub local: bool,
    pub public_address: String,
    pub title: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Event {
    #[serde(rename = "library.on.deck")]
    LibraryOnDeck,
    #[serde(rename = "library.new")]
    LibraryNew,
    #[serde(rename = "media.pause")]
    MediaPause,
    #[serde(rename = "media.play")]
    MediaPlay,
    #[serde(rename = "media.rate")]
    MediaRate,
    #[serde(rename = "media.resume")]
    MediaResume,
    #[serde(rename = "media.scrobble")]
    MediaScrobble,
    #[serde(rename = "media.stop")]
    MediaStop,
    #[serde(rename = "admin.database.backup")]
    AdminDatabaseBackup,
    #[serde(rename = "admin.database.corrupted")]
    AdminDatabaseCorrupted,
    #[serde(rename = "device.new")]
    DeviceNew,
    #[serde(rename = "playback.started")]
    PlaybackStarted,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Payload {
    pub event: Event,
    pub user: bool,
    pub owner: bool,
    #[serde(rename(deserialize = "Account"))]
    pub account: Account,
    #[serde(rename(deserialize = "Server"))]
    pub server: Server,
    #[serde(rename(deserialize = "Player"))]
    pub player: Player,
    #[serde(rename(deserialize = "Metadata"))]
    pub metadata: Option<HashMap<String, Value>>,
}

pub async fn handle_webhook(form: FormData) -> Result<impl warp::Reply, warp::Rejection> {
    let parts: Vec<Part> = form
        .try_collect()
        .await
        .map_err(|_e| warp::reject::reject())?;

    let mut payload = None;
    let mut thumbs = Vec::new();

    // Split parts of multipart form
    for p in parts {
        match p.name() {
            "payload" => {
                // Fold stream that makes up the body into a vec
                let value = p
                    .stream()
                    .try_fold(Vec::new(), |mut vec, data| {
                        vec.put(data);
                        async move { Ok(vec) }
                    })
                    .await
                    .map_err(|_e| warp::reject::reject())?;

                // Sometimes plex messes up and passes us an image with the payload title
                // Attempt parsing as JSON, else assume the buffer is an image
                // TODO: Find some way to validate it is JPEG data
                if let Ok(payload_part) = serde_json::from_slice::<Payload>(&value) {
                    payload = Some(payload_part);
                } else {
                    thumbs.push(value);
                }
            }
            "thumb" => {
                let value = p
                    .stream()
                    .try_fold(Vec::new(), |mut vec, data| {
                        vec.put(data);
                        async move { Ok(vec) }
                    })
                    .await
                    .map_err(|_e| warp::reject::reject())?;
                thumbs.push(value);
            }
            s => {
                println!("unexpected pattern {}", s);
            }
        }
    }

    //TODO: Properly deconstruct and match events
    if let Some(p) = payload {
        info!("Got request, user {}, event {:?}", p.account.title, p.event);
    }

    //TODO: send thumbnails to the right place
    for (i, t) in thumbs.iter().enumerate() {
        let mut f = std::fs::File::create(format!("thumb{}.jpeg", i)).unwrap();
        f.write_all(t).unwrap();
    }

    Ok(warp::reply())
}
