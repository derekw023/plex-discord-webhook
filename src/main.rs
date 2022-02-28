use color_eyre::Report;
use tokio::join;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use chrono::prelude::*;
use futures::future::join_all;
use std::io::Write;
use std::path;
use std::time::{Duration, SystemTime};
use std::{collections::HashMap, fs};

mod discord;
mod plex;

use warp::Filter;
const MAX_LENGTH: u64 = 1024 * 1024;

use plex::webhook::PlexWebhookRequest;

use clap::Parser;

use crate::discord::webhook::{Embed, EmbedAuthor, EmbedFooter, WebhookExecutor};

#[derive(Parser, Clone)]
struct Config {
    /// Webhook URL to post to, may be specified multiple times
    #[clap(short)]
    webhook_urls: Vec<String>,

    /// Port to listen on, default 8001
    #[clap(default_value = "8001")]
    port: u16,

    /// Save requests to a log folder
    #[clap(short)]
    save_requests: bool,

    /// Throttle notifications for siblings to this many seconds between pings
    #[clap(short, default_value = "0")]
    throttle: u32,
}

// Current thread scheduler to minimize overhead, and this should really all fit on one anyway
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Report> {
    setup()?;

    let args = Config::parse();

    // Save requests from plex just 'cause
    if args.save_requests {
        let path = path::Path::new("./logs/");

        if !path.exists() || !path.is_dir() {
            fs::create_dir(path)?;
        }
    }

    // Buffer to hold plex request queue
    let (tx, mut rx) = tokio::sync::mpsc::channel(1024);

    // Buffer to transfer rate limited messages
    let (rate_limit_tx, mut rate_limit_rx) = tokio::sync::mpsc::channel(1024);

    // Internally this uses an Arc<Mutex<T>>, so cloning directly is cheap and safe
    let discord_client = discord::webhook::WebhookExecutor::new();

    // Accept and parse the webhook request and send it to a mpsc channel
    let api = warp::path("plex")
        .and(warp::post())
        // .and(log_body())
        .and(warp::filters::multipart::form().max_length(MAX_LENGTH))
        .and_then(plex::webhook::handle_webhook)
        // I feel like this clone should be rolled into the next closure but I'm not sure the syntax feature exists
        .map(move |msg| (msg, tx.clone()))
        .then(|arg: (PlexWebhookRequest, Sender<_>)| async {
            let (msg, tx) = arg;

            // Push the message onto a channel, with a check that the receiving end lives
            match tx.send(msg).await {
                Ok(()) => warp::http::StatusCode::OK,
                Err(e) => {
                    error!("Attempted to send webhook message to a dead channel with error {e}");

                    // Channel is shut down, server should stop accepting requests. But for now just error 500
                    // If the app is shutting down, warp should be signalled to stop
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR
                }
            }
        });

    // Serve the API defined above
    let server_future = warp::serve(api).run(([0, 0, 0, 0], args.port));

    // Process received plex messages in one place, to allow combination and filtering of them
    let messager_future = |args: Config, discord_client: WebhookExecutor| async move {
        // Initialize a message template to clone for all further messages
        let mut default_embed = Embed::default();
        default_embed.author = Some(EmbedAuthor {
            name: "derekw023/plex-discord-webhook".into(),
            url: Some("https://github.com/derekw023/plex-discord-webhook".into()),
            icon_url: Some("https://github.githubassets.com/favicons/favicon.svg".into()),
            proxy_icon_url: None,
        });
        default_embed.footer = Some(EmbedFooter {
            text: "Submit feature requests/bug reports on github".into(),
            icon_url: Some("https://github.githubassets.com/favicons/favicon.svg".into()),
            proxy_icon_url: None,
        });
        default_embed.url = Some("https://github.com/derekw023/plex-discord-webhook".into());

        // Receive messages while there are publishers to the channel
        while let Some(msg) = rx.recv().await {
            // Save message if directed to
            if args.save_requests {
                // Come up with a name from a timestamp
                let now = Utc::now();
                let path = format!("./logs/{:?} - {now}.json", msg.payload.event);
                let thumbpath = format!("./logs/{now}.jpeg");
                let f = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(path)
                    .unwrap();

                if let Some(thumb) = msg.thumb {
                    let mut thumbfile = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(thumbpath)
                        .unwrap();

                    thumbfile.write_all(&thumb).unwrap();
                }

                serde_json::to_writer_pretty(f, &msg.payload).unwrap();
            }

            // Only handle library add events for now
            // if let plex::models::Event::LibraryNew = msg.payload.event {
            if let Some(metadata) = msg.payload.metadata {
                debug!("{metadata:#?}");
                let mut embeds = Vec::new();

                let mut em = default_embed.clone();

                // Construct a message title from media metadata
                let mut message_title = format!(
                    "New {} added",
                    metadata
                        .media_type
                        .clone()
                        .unwrap_or_else(|| "media".to_string())
                );
                let mut message_description = String::new();

                // Gracefully fall through, and fill in context based on what kind of content this is
                if let Some(grandparent_title) = &metadata.grandparent_title {
                    // has grandparent title, is a tv episode with associated season (parent) and show (this)
                    message_title += &format!(": {grandparent_title}");
                    if let Some(parent_title) = &metadata.parent_title {
                        // Append season context
                        message_title += &format!(" - {parent_title}");

                        // Construct a description from media type, title and number
                        if let Some(media_type) = metadata.media_type {
                            message_description += &format!("{media_type} ");
                        }

                        // This will be the episode number for TV episodes
                        if let Some(index) = metadata.index {
                            message_description += &format!("{index}");
                        }

                        // Add episode title as description
                        if let Some(title) = &metadata.title {
                            message_description += &format!(": {title}");
                        }
                    }
                } else if let Some(parent_title) = &metadata.parent_title {
                    // no grandparent title, this item refers to a season of a show, or a show without seasons?
                    message_title += &format!(": {parent_title}");

                    if let Some(title) = &metadata.title {
                        // Append season context
                        message_title += &format!(" - {title}");

                        // Construct a description from media type, title and number
                        if let Some(media_type) = metadata.media_type {
                            message_description += &format!("{media_type} ");
                        }

                        // This will be the episode number for TV episodes
                        if let Some(index) = metadata.index {
                            message_description += &format!("{index}");
                        }
                    }
                } else if let Some(title) = &metadata.title {
                    message_title += &format!(": {title}");
                } else {
                    error!("Metadata has no title... sending empty message");
                }
                // TODO: Find pictures

                // Move into embed object
                em.title = Some(message_title);
                em.description = if message_description.is_empty() {
                    None
                } else {
                    Some(message_description)
                };

                // Build a hash to uniquely ID this item's parents, if any
                let mut hash = String::new();
                if let Some(grandparent) = &metadata.grandparent_title {
                    hash += grandparent;
                }
                if let Some(parent) = &metadata.parent_title {
                    hash += parent;
                }

                // Time throttle things if configured to, and if this should be throttled
                if args.throttle > 0 && !hash.is_empty() {
                    rate_limit_tx.send((hash, em)).await.unwrap();
                } else {
                    // Add the embed to list to send
                    embeds.push(em);
                }

                // Send something if there is something to send
                if !embeds.is_empty() {
                    // Wrap the embeds we made in a request object
                    let request = discord::webhook::WebhookRequest::Embeds(embeds);

                    // Execute the request against each webhook URL concurrently
                    join_all(
                        args.webhook_urls
                            .iter()
                            .map(|url| request.execute(discord_client.clone(), url)),
                    )
                    .await;
                }
            } else {
                error!("Received a library new event without metadata, nothing to notify with");
            }
            // }
        }
    };

    // This should be refactored into the above future
    //TODO: Do th^s
    let rate_limiter_future = |args: Config, discord_client: WebhookExecutor| async move {
        // Initialize a hashmap to manage a queue of sorts for rate limiting messages
        let mut parents_map: HashMap<String, (tokio::time::Instant, Vec<Embed>)> = HashMap::new();
        let mut oldest_ts = tokio::time::Instant::now();

        let mut pending_requests = Vec::new();

        loop {
            let now = tokio::time::Instant::now();

            // Schedule a wakeup on oldest_ts
            let ratelimit_delay =
                tokio::time::sleep_until(oldest_ts + Duration::from_secs(args.throttle.into()));
            tokio::pin!(ratelimit_delay);

            // wake up on the sooner of: something comes in on the channel or timer expires
            tokio::select! {
                recvd = rate_limit_rx.recv() => {
                    if let Some((hash, em)) = recvd {
                        if let Some(val) = parents_map.get_mut(&hash) {
                            let (ts, embeds) = val;
                            *ts = now;
                            embeds.push(em);
                        } else {
                            // Initialize the item to just this pending message
                            parents_map.insert(hash, (now, vec![em]));
                        }
                    } else {
                        // End execution of this future if no senders exist
                        return;
                    }
                },
                // disable if no pending messages
                _ = &mut ratelimit_delay, if !parents_map.is_empty()=> {
                    // Iterate all elements in the map and send ones that are old enough
                    parents_map = parents_map
                    .into_iter()
                    .filter_map(|arg| {
                        let (key, (ts, ems)) = arg;

                        // Send if old enough
                        if now.duration_since(ts)
                            >= std::time::Duration::from_secs(args.throttle.into())
                        {
                            // Basically, collapse all existing embeds into one and pop from hashmap
                            // Stack descriptions up with newlines in between but just copy the first embed for all other fields
                            let desc = ems.iter().fold(String::new(), |mut d, l| {
                                d += l.description.as_ref().unwrap();
                                d += "\n";
                                d
                            });

                            let mut whole_embed = ems[0].clone();
                            whole_embed.description = Some(desc);

                            // Push the request to execute onto a queue of requests
                            let request = discord::webhook::WebhookRequest::Embeds(vec![whole_embed]);
                            pending_requests.push(request);

                            // Finally, remove from the hashmap
                            None
                        } else {
                            // Update oldest ts
                            oldest_ts = now.min(ts);
                            // else, keep for next pass
                            Some((key, (ts, ems)))
                        }
                    })
                    .collect();
                }
            };

            // Send everything that is ready to send
            while let Some(req) = pending_requests.pop() {
                // Execute the request against each webhook URL concurrently
                join_all(
                    args.webhook_urls
                        .iter()
                        .map(|url| req.execute(discord_client.clone(), url)),
                )
                .await;
            }
        }
    };

    info!("Starting up plex webhook relay");
    join!(
        messager_future(args.clone(), discord_client.clone()),
        server_future,
        rate_limiter_future(args.clone(), discord_client.clone())
    );
    Ok(())
}

fn setup() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    // For now, debug at top level and info for all other modules and crates. Will change to warning later
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            "plex_discord_webhook=info,plex_discord_webhook::plex=info,plex_discord_webhook::discord=info",
        )
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}
