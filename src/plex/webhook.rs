use std::io::Write;

use bytes::BufMut;
use futures::TryStreamExt;
use tracing::{error, info};
use warp::multipart::{FormData, Part};

use std::sync::Arc;
use tokio::sync::Mutex;

use super::models::Payload;

pub struct PlexHandler {
    pub req_count: u32,
}

impl PlexHandler {
    pub fn new() -> Self {
        Self { req_count: 0 }
    }
}

pub async fn handle_webhook(
    ctx: Arc<Mutex<PlexHandler>>,
    form: FormData,
) -> Result<impl warp::Reply, warp::Rejection> {
    let parts: Vec<Part> = form
        .try_collect()
        .await
        .map_err(|_e| warp::reject::reject())?;

    let mut payload = None;
    let mut thumbs = None;
    {
        let mut c = ctx.lock().await;
        c.req_count += 1;
    }
    // Split parts of multipart form
    for p in parts {
        match p.name() {
            "payload" => {
                // Fold stream that makes up the body into a vec
                let value = p
                    .stream()
                    .try_fold(Vec::new(), |mut vec, data| {
                        vec.put(data);
                        async move { Ok(vec) }
                    })
                    .await
                    .map_err(|_e| warp::reject::reject())?;

                // Sometimes plex messes up and passes us an image with the payload title
                // Attempt parsing as JSON, else assume the buffer is an image
                // TODO: Find some way to validate it is JPEG data
                let payload_part = serde_json::from_slice::<Payload>(&value).map_err(|e| {
                    error!("Failed to parse request payload with {}", e);
                    warp::reject()
                })?;
                payload = Some(payload_part);
            }
            "thumb" => {
                let value = p
                    .stream()
                    .try_fold(Vec::new(), |mut vec, data| {
                        vec.put(data);
                        async move { Ok(vec) }
                    })
                    .await
                    .map_err(|_e| warp::reject::reject())?;
                thumbs = Some(value);
            }
            s => {
                println!("unexpected pattern {}", s);
            }
        }
    }

    if let Some(p) = payload {
        let c = ctx.lock().await;
        info!(
            "Got request #{}, user {}, event {:?}",
            c.req_count, p.account.title, p.event
        );
    }

    //TODO: send thumbnails to the right place
    if let Some(t) = thumbs {
        let mut f = std::fs::File::create("thumb.jpeg").unwrap();
        f.write_all(&t).unwrap();
    }

    Ok(warp::reply())
}
