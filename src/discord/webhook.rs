use warp::hyper::{body::to_bytes, client::HttpConnector, Body, Client, Request};

use hyper_tls::HttpsConnector;
use warp::hyper::http;

use serde::Serialize;

use tracing::{debug, error};

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

    /// Execute the given request against the passed webhook URL
    ///
    /// # Arguments
    ///
    ///  * `url` - Webhook URL acquired from a Discord server's API or admin interface
    ///
    ///  * `request` - A message to post to the specified channel, must be constructed elsewhere
    pub async fn execute_webhook(&self, url: &str, request: &WebhookRequest) -> impl warp::Reply {
        let body = Body::from(serde_json::to_string(request).unwrap());

        let req = Request::post(url)
            .header("Content-Type", "application/json")
            .body(body)
            .unwrap();

        debug!("{:?}", req);

        let mut resp = self.client.request(req).await.unwrap();

        debug!("Discord webhook reply status: {}", resp.status());

        // This is expected to be status 204, no content. If there is content format and log it
        if !http::StatusCode::is_success(&resp.status()) {
            let body_bytes = to_bytes(resp.body_mut()).await.unwrap();
            let body_str = std::string::String::from_utf8_lossy(&body_bytes);

            error!("Server replied with {}", body_str);
        }

        warp::reply()
    }
}

#[derive(Debug, Serialize)]
struct Embed {
    title: Option<String>,
    /// Type should always be rich for webhooks, and in general
    #[serde(rename = "type")]
    kind: Option<String>,
    description: Option<String>,
    url: Option<String>,
    timestamp: Option<String>,
    color: Option<u32>,
    footer: Option<EmbedFooter>,
    image: Option<EmbedMedia>,
    thumbnail: Option<EmbedMedia>,
    video: Option<EmbedMedia>,
    provider: Option<EmbedProvider>,
    author: Option<EmbedAuthor>,
    fields: Option<Vec<EmbedField>>,
}

#[derive(Debug, Serialize)]
struct EmbedFooter {
    test: String,
    /// HTTPS link to an icon image
    icon_url: Option<String>,
    /// Proxied URL to the icon (not sure what this is for)
    proxy_icon_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct EmbedMedia {
    url: String,

    proxy_url: Option<String>,
    height: Option<u32>,
    width: Option<u32>,
}

#[derive(Debug, Serialize)]
struct EmbedProvider {
    name: String,
    url: Option<String>,
}

#[derive(Debug, Serialize)]
struct EmbedAuthor {
    name: String,
    url: Option<String>,
    icon_url: Option<String>,
    proxy_icon_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct EmbedField {
    name: String,
    value: String,
    inline: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Serialize)]
pub struct WebhookRequest {
    pub content: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub tts: Option<bool>,
    embeds: Option<Vec<Embed>>,
    allowed_mentions: Option<AllowedMention>,
    // Other fields are available per API docs, but are not implemented as for the most part they don't apply to webhooks or this use case
}

impl WebhookRequest {
    pub fn new() -> Self {
        Self {
            content: Some("Hello Discord".into()),
            username: None,
            avatar_url: None,
            tts: None,
            embeds: None,
            allowed_mentions: None,
        }
    }
}
