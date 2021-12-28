use color_eyre::Report;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod plex;

use std::sync::Arc;
use tokio::sync::Mutex;

use warp::Filter;
const MAX_LENGTH: u64 = 1024 * 1024;

#[tokio::main]
async fn main() -> Result<(), Report> {
    setup()?;

    let port: u16 = 8000;

    let plex_handler = Arc::new(Mutex::new(plex::webhook::PlexHandler::new()));

    let api = warp::path("plex")
        .and(warp::post())
        .map(move || plex_handler.clone())
        .and(warp::filters::multipart::form().max_length(MAX_LENGTH))
        .and_then(plex::webhook::handle_webhook);

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
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}
