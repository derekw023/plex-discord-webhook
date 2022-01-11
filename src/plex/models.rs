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
pub struct Credit {
    pub filter: String,
    pub id: u32,
    pub tag: String,
    pub role: Option<String>,
    pub thumb: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Link {
    id: String,
}

// The plex webhook docs say nothing of significance that guarantees the presence or absence of these fields
//  To avoid errors, every field is optional in metadata, and errors resulting from missing data should be handled on a case-by-case basis
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    // Child info (directly describing this item)
    pub title: Option<String>,
    pub title_sort: Option<String>,
    pub thumb: Option<String>,
    pub key: Option<String>,
    pub guid: Option<String>,
    pub rating_key: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "Guid")]
    pub external_links: Option<Vec<Link>>,
    #[serde(rename = "type")]
    pub media_type: Option<String>,

    // Miscellaneous extra info
    pub index: Option<u64>,
    pub art: Option<String>,
    pub skip_count: Option<u64>,
    pub view_count: Option<u64>,
    pub audience_rating: Option<f32>,
    pub audience_rating_image: Option<String>,
    pub library_section_type: Option<String>,
    pub content_rating: Option<String>,
    pub view_offset: Option<u64>,

    // Credits info
    #[serde(rename = "Writer")]
    pub writer: Option<Vec<Credit>>,
    #[serde(rename = "Director")]
    pub director: Option<Vec<Credit>>,
    #[serde(rename = "Role")]
    pub role: Option<Vec<Credit>>,
    #[serde(rename = "Producer")]
    pub producer: Option<Vec<Credit>>,

    // Times, I think. Not sure exactly what format these timestamps are in
    pub originally_available_at: Option<String>,
    pub updated_at: Option<u64>,
    pub last_viewed_at: Option<u64>,
    pub duration: Option<u64>,
    pub added_at: Option<u64>,

    // Parent info (if present)
    pub parent_rating_key: Option<String>,
    pub parent_index: Option<u64>,
    pub parent_key: Option<String>,
    pub parent_title: Option<String>,
    pub parent_guid: Option<String>,
    pub parent_thumb: Option<String>,

    // Grandparent info (if present)
    pub grandparent_key: Option<String>,
    pub grandparent_title: Option<String>,
    pub grandparent_thumb: Option<String>,
    pub grandparent_theme: Option<String>,
    pub grandparent_guid: Option<String>,
    pub grandparent_rating_key: Option<String>,
    pub grandparent_art: Option<String>,

    // Containing library info
    pub library_section_title: Option<String>,
    pub library_section_key: Option<String>,
    #[serde(rename = "librarySectionID")]
    pub library_section_id: u32,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
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
    pub player: Option<Player>,
    #[serde(rename(deserialize = "Metadata"))]
    pub metadata: Option<Metadata>,
}
