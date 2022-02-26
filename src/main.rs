use color_eyre::Report;
use tokio::join;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use futures::future::join_all;

mod discord;
mod plex;

use warp::Filter;
const MAX_LENGTH: u64 = 1024 * 1024;

use plex::webhook::PlexWebhookRequest;

use clap::Parser;

use crate::discord::webhook::Embed;

#[derive(Parser)]
struct Config {
    /// Webhook URL to post to, may be specified multiple times
    #[clap(short)]
    webhook_urls: Vec<String>,

    /// Port to listen on, default 8001
    #[clap(default_value = "8001")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    setup()?;

    let args = Config::parse();

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    // Internally this uses an Arc<Mutex<T>>, so cloning directly is cheap and safe
    let discord_client = discord::webhook::WebhookExecutor::new();

    // Accept and parse the webhook request and send it to a mpsc channel
    let api = warp::path("plex")
        .and(warp::post())
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
    let messager_future = async move {
        // Receive messages while there are publishers to the channel
        while let Some(msg) = rx.recv().await {
            use plex::models::Event::*;

            // Only handle library add events for now
            if let LibraryNew = msg.payload.event {
                let mut embeds = Vec::new();

                let mut em = Embed::default();
                em.title = Some("Media Added!".into());

                embeds.push(em);

                // for each media item added, construct and send a message
                let ping = discord::webhook::WebhookRequest::Embeds(embeds);

                // Send the request for each webhook url configured
                let requests: Vec<_> = args
                    .webhook_urls
                    .iter()
                    .map(|url| ping.execute(discord_client.clone(), url))
                    .collect();

                join_all(requests).await;
            }
        }
    };

    info!("Starting up plex webhook handler");
    join!(messager_future, server_future);
    Ok(())
}

fn setup() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            "plex_discord_webhook=info,plex_discord_webhook::plex::webhook=info",
        )
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}
