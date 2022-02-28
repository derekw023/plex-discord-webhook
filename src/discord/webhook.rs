use warp::hyper::{body::to_bytes, client::HttpConnector, Body, Client, Request};

use hyper_tls::HttpsConnector;
use warp::hyper::http;

use serde::Serialize;

use color_eyre::{eyre::eyre, Result};
use tracing::debug;

/// At this point just a wrapper around an HTTP client
#[derive(Debug, Clone)]
pub struct WebhookExecutor {
    /// Only support https transport, as the discord API is HTTPS only
    client: Client<HttpsConnector<HttpConnector>>,
}

impl WebhookExecutor {
    /// This initializes a new HTTP Client, can be cloned very cheaply for sharing the underlying client's connection pool
    pub fn new() -> Self {
        Self {
            client: Client::builder().build(HttpsConnector::new()),
        }
    }
}

#[derive(Debug, Serialize, Default, Clone)]
pub struct Embed {
    pub title: Option<String>,
    /// Type should always be rich for webhooks, and in general
    #[serde(rename = "type")]
    kind: EmbedKind,
    pub description: Option<String>,
    pub url: Option<String>,
    pub timestamp: Option<String>,
    pub color: Option<u32>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<EmbedMedia>,
    pub thumbnail: Option<EmbedMedia>,
    pub video: Option<EmbedMedia>,
    pub provider: Option<EmbedProvider>,
    pub author: Option<EmbedAuthor>,
    pub fields: Option<Vec<EmbedField>>,
}

#[derive(Debug, Serialize, Clone, Copy)]
enum EmbedKind {
    #[serde(rename = "rich")]
    Rich,
}

impl Default for EmbedKind {
    fn default() -> Self {
        Self::Rich
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct EmbedFooter {
    pub text: String,
    /// HTTPS link to an icon image
    pub icon_url: Option<String>,
    /// Proxied URL to the icon (not sure what this is for)
    pub proxy_icon_url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EmbedMedia {
    url: String,

    proxy_url: Option<String>,
    height: Option<u32>,
    width: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EmbedProvider {
    name: String,
    url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EmbedAuthor {
    pub name: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub proxy_icon_url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EmbedField {
    name: String,
    value: String,
    inline: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum AllowedMentionType {
    Roles,
    Users,
    Everyone,
}

#[derive(Debug, Serialize)]
struct AllowedMention {
    parse: Vec<AllowedMentionType>,
    roles: Vec<String>,
    users: Vec<String>,
    replied_user: bool,
}

/// May be constructed to specify optional flags that can be sent alongside the main request
#[derive(Serialize, Debug)]
pub struct RequestMetadata {
    username: Option<String>,
    avatar_url: Option<String>,
    tts: Option<bool>,
    allowed_mentions: Option<AllowedMention>,
}

/// May be constructed as a plain text message or a rich embed struct
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum WebhookRequest {
    Content(String),
    Embeds(Vec<Embed>),
}

impl WebhookRequest {
    pub async fn execute(&self, client: WebhookExecutor, url: &str) -> Result<()> {
        let body = Body::from(serde_json::to_string(self).unwrap());

        let req = Request::post(url)
            .header("Content-Type", "application/json")
            .body(body)?;

        debug!("{:?}", req);

        let mut resp = client.client.request(req).await?;

        debug!("Discord webhook reply status: {}", resp.status());

        // This is expected to be status 204, no content. If there is content format and log it
        if !http::StatusCode::is_success(&resp.status()) {
            let body_bytes = to_bytes(resp.body_mut()).await?;
            let body_str = std::string::String::from_utf8_lossy(&body_bytes);

            Err(eyre!("Server replied with {}", body_str))
        } else {
            Ok(())
        }
    }
}
