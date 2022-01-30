use bytes::BufMut;
use futures::TryStreamExt;
use tracing::{debug, error, warn};
use warp::multipart::{FormData, Part};

use super::models::Payload;

/// A webhook request from a Plex server is comprised of two parts, a [Payload] and an optional thumbnail
/// for certain events. The thumbnail is JPEG encoded, stored here in a [Vec].
pub struct PlexWebhookRequest {
    pub payload: Payload,
    pub thumb: Option<Vec<u8>>,
}

/// Given a multipart form submitted by a plex server, attempt to parse as a plex webhook message
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
                // Take the thumbnail and just shove it into a byte vector, dependent code may use it or not
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
                warn!("Discarding unexpected form field {s}")
            }
        }
    }

    if let Some(p) = payload {
        if let Some(t) = thumbs {
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
