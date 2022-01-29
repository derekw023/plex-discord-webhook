use color_eyre::Report;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

mod discord;
mod plex;

use std::sync::Arc;
use tokio::sync::Mutex;

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

    let plex_handler = Arc::new(Mutex::new(plex::webhook::PlexHandler::new()));

    // Start with an empty base client
    let discord_client_base = discord::webhook::WebhookExecutor::new("".into());

    // Clone it for all subsequent clients
    let discord_clients: Vec<discord::webhook::WebhookExecutor> = args
        .webhook_urls
        .iter()
        .map(|u| discord_client_base.clone_with_url(u.into()))
        .collect();

    drop(discord_client_base);

    let api = warp::path("plex")
        .and(warp::post())
        .map(move || plex_handler.clone())
        .and(warp::filters::multipart::form().max_length(MAX_LENGTH))
        .and_then(plex::webhook::handle_webhook)
        .map(move |msg| (msg, discord_clients.clone()))
        .then(
            |arg: (PlexWebhookRequest, Vec<discord::webhook::WebhookExecutor>)| async move {
                let msg = arg.0;
                let client = arg.1;

                let _message = format!(
                    "User {} {:?}'ed {}",
                    msg.payload.account.title,
                    msg.payload.event,
                    msg.payload.metadata.unwrap().title.unwrap()
                );

                let content = discord::webhook::WebhookRequest::new();
                client.iter().map(|c| async {
                    c.execute_webhook(&content).await;
                });

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
            "plex_discord_webhook=info,plex_discord_webhook::plex::webhook=debug",
        )
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}
