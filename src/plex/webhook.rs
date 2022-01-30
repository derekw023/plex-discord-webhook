use bytes::BufMut;
use futures::TryStreamExt;
use std::io::Write;
use tracing::{debug, error, info, warn};
use warp::multipart::{FormData, Part};

use futures::TryFuture;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::models::Payload;

pub struct PlexWebhookRequest {
    pub payload: Payload,
    pub thumb: Option<Vec<u8>>,
}

pub async fn handle_webhook(form: FormData) -> Result<PlexWebhookRequest, warp::Rejection> {
    let parts: Vec<Part> = form
        .try_collect()
        .await
        .map_err(|_e| warp::reject::reject())?;

    let mut payload = None;
    let mut thumbs = None;
    // Split parts of multipart form
    for p in parts {
        match p.name() {
            "payload" => {
                // Fold stream that makes up the body into a vec for deserialization
                let value = p
                    .stream()
                    .try_fold(Vec::new(), |mut vec, data| {
                        vec.put(data);
                        async move { Ok(vec) }
                    })
                    .await
                    .map_err(|_e| warp::reject::reject())?;

                // Parse payload using models and serde_json
                let payload_part = serde_json::from_slice::<Payload>(&value).map_err(|e| {
                    error!("Failed to parse request payload with {}", e);
                    warp::reject()
                })?;

                // Warn if metadata parsing is wrong, necessary since the format may change and was gleaned from reverse-engineering in the first place
                if let Some(metadata) = payload_part.metadata.as_ref() {
                    if !metadata.extra.is_empty() {
                        warn!("{} extra fields in metadata", metadata.extra.len());
                        debug!("Extra metadata fields: {:#?}", metadata.extra);
                    }
                }

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
        info!(
            "Got request #{}, user {}, event {:?}",
            reqcount, p.account.title, p.event
        );

        let f = std::fs::File::create(format!("logs/req{}.json", reqcount)).unwrap();

        serde_json::to_writer(f, &p).unwrap();

        if let Some(t) = thumbs {
            let mut f = std::fs::File::create(format!("logs/thumb{}.jpeg", reqcount)).unwrap();
            f.write_all(&t).unwrap();

            Ok(PlexWebhookRequest {
                payload: p,
                thumb: Some(t),
            })
        } else {
            Ok(PlexWebhookRequest {
                payload: p,
                thumb: None,
            })
        }
    } else {
        //TODO: reply with proper error code
        Err(warp::reject())
    }
}
