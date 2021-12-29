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
    filter: String,
    id: u32,
    tag: String,
    role: Option<String>,
    thumb: Option<String>,
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
    title: Option<String>,
    title_sort: Option<String>,
    thumb: Option<String>,
    key: Option<String>,
    guid: Option<String>,
    rating_key: Option<String>,
    summary: Option<String>,
    #[serde(rename = "Guid")]
    external_links: Option<Vec<Link>>,
    #[serde(rename = "type")]
    media_type: Option<String>,

    // Miscellaneous extra info
    index: Option<u64>,
    art: Option<String>,
    skip_count: Option<u64>,
    view_count: Option<u64>,
    audience_rating: Option<f32>,
    audience_rating_image: Option<String>,
    library_section_type: Option<String>,
    content_rating: Option<String>,
    view_offset: Option<u64>,

    // Credits info
    #[serde(rename = "Writer")]
    writer: Option<Vec<Credit>>,
    #[serde(rename = "Director")]
    director: Option<Vec<Credit>>,
    #[serde(rename = "Role")]
    role: Option<Vec<Credit>>,

    // Times, I think. Not sure exactly what format these timestamps are in
    originally_available_at: Option<String>,
    updated_at: Option<u64>,
    last_viewed_at: Option<u64>,
    duration: Option<u64>,
    added_at: Option<u64>,

    // Parent info (if present)
    parent_rating_key: Option<String>,
    parent_index: Option<u64>,
    parent_key: Option<String>,
    parent_title: Option<String>,
    parent_guid: Option<String>,
    parent_thumb: Option<String>,

    // Grandparent info (if present)
    grandparent_key: Option<String>,
    grandparent_title: Option<String>,
    grandparent_thumb: Option<String>,
    grandparent_theme: Option<String>,
    grandparent_guid: Option<String>,
    grandparent_rating_key: Option<String>,
    grandparent_art: Option<String>,

    // Containing library info
    library_section_title: Option<String>,
    library_section_key: Option<String>,
    #[serde(rename = "librarySectionID")]
    library_section_id: u32,

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
    pub player: Player,
    #[serde(rename(deserialize = "Metadata"))]
    pub metadata: Option<Metadata>,
}
