use color_eyre::Report;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

mod discord;
mod plex;

use std::sync::Arc;

use warp::Filter;
const MAX_LENGTH: u64 = 1024 * 1024;

use plex::webhook::PlexWebhookRequest;

use clap::Parser;

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

    // Internally this uses an Arc<Mutex<T>>, so cloning directly is cheap and safe
    let discord_client = discord::webhook::WebhookExecutor::new();

    // Arc to avoid copying webhook strings
    let webhook_urls = Arc::new(args.webhook_urls);

    let api = warp::path("plex")
        .and(warp::post())
        .and(warp::filters::multipart::form().max_length(MAX_LENGTH))
        .and_then(plex::webhook::handle_webhook)
        // Clone ARCs for state structures
        .map(move |msg| (msg, discord_client.clone(), webhook_urls.clone()))
        .then(
            |arg: (
                PlexWebhookRequest,
                discord::webhook::WebhookExecutor,
                Arc<Vec<String>>,
            )| async {
                let msg = arg.0;
                let client = arg.1;
                let urls = arg.2;

                let message = discord::webhook::WebhookRequest::Content(format!(
                    "User {} {:?}'ed {}",
                    msg.payload.account.title,
                    msg.payload.event,
                    msg.payload.metadata.unwrap().title.unwrap()
                ));

                for url in urls.iter() {
                    let ret = message.execute(client.clone(), url).await;
                    if ret.is_err() {
                        error!("Request failed because: {ret:?}");
                    }
                }
                warp::reply()
            },
        );

    let server_future = warp::serve(api).run(([0, 0, 0, 0], args.port));

    info!("Starting up plex webhook handler");
    server_future.await;

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
