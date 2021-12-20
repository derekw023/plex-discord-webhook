use bytes::BufMut;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use warp::multipart::{FormData, Part};
use warp::Filter;

const MAX_LENGTH: u64 = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub struct Account {
    pub id: u64,
    pub thumb: String,
    pub title: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Server {
    pub title: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub local: bool,
    pub public_address: String,
    pub title: String,
    pub uuid: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Payload {
    pub event: String,
    pub user: bool,
    pub owner: bool,
    #[serde(rename(deserialize = "Account"))]
    pub account: Account,
    #[serde(rename(deserialize = "Server"))]
    pub server: Server,
    #[serde(rename(deserialize = "Player"))]
    pub player: Player,
    #[serde(rename(deserialize = "Metadata"), flatten)]
    pub metadata: Option<String>,
}

#[tokio::main]
async fn main() {
    let port: u16 = 8000;

    let api = warp::path("plex")
        .and(warp::post())
        .and(warp::filters::multipart::form().max_length(MAX_LENGTH))
        .and_then(handle_webhook);

    let server_future = warp::serve(api).run(([127, 0, 0, 1], port.clone()));

    server_future.await;
}

pub async fn handle_webhook(form: FormData) -> Result<impl warp::Reply, warp::Rejection> {
    let parts: Vec<Part> = form
        .try_collect()
        .await
        .map_err(|_e| warp::reject::reject())?;

    for p in parts {
        if p.name() != "payload" {
            println!("Skipping non-payload form part");
        }

        let value = p
            .stream()
            .try_fold(Vec::new(), |mut vec, data| {
                vec.put(data);
                async move { Ok(vec) }
            })
            .await
            .map_err(|_e| warp::reject::reject())?;

        let json = serde_json::from_slice::<Payload>(&value)
            .map_err(|e| println!("Failed to parse payload with {:?}", e));
        println!("{:#?}", json);
    }

    Ok(warp::reply())
}
