use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
