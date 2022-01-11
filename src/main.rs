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

#[tokio::main]
async fn main() -> Result<(), Report> {
    setup()?;

    let port: u16 = 8001;
    let webhook_url: String = "DUMMY".to_string();

    let plex_handler = Arc::new(Mutex::new(plex::webhook::PlexHandler::new()));

    let discord_client = Arc::new(discord::webhook::WebhookExecutor::new(webhook_url));

    let api = warp::path("plex")
        .and(warp::post())
        .map(move || plex_handler.clone())
        .and(warp::filters::multipart::form().max_length(MAX_LENGTH))
        .and_then(plex::webhook::handle_webhook)
        .map(move |msg| (msg, discord_client.clone()))
        .then(
            |arg: (PlexWebhookRequest, Arc<discord::webhook::WebhookExecutor>)| async move {
                let msg = arg.0;
                let client = arg.1;

                let message = format!(
                    "User {} {:?}'ed {}",
                    msg.payload.account.title,
                    msg.payload.event,
                    msg.payload.metadata.unwrap().title.unwrap()
                );

                let content = discord::webhook::WebhookRequest { content: message };
                client.clone().execute_webhook(content).await
            },
        );

    let server_future = warp::serve(api).run(([0, 0, 0, 0], port));

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
            "plex_discord_webhook=info,plex_discord_webhook::plex=debug",
        )
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}
