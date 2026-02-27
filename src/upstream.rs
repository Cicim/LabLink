use std::time::Duration;

use reqwest::{Client, Response};
use serde::de::DeserializeOwned;

use crate::errors::{LinkError, LinkResult};

/// Build the shared `reqwest` client.
/// Call this once at startup and pass the resulting `Client` to your handlers via axum `State`.
pub fn build_client() -> anyhow::Result<Client> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .build()?;
    Ok(client)
}

/// Perform a GET request to `url` and deserialize the JSON body into `T`.
/// Connection failures and bad status codes are mapped to `LinkError` variants.
pub async fn get_json<T: DeserializeOwned>(client: &Client, url: &str) -> LinkResult<T> {
    let response = client.get(url).send().await.map_err(LinkError::from)?;
    _handle_json_response(response).await
}

/// Perform a POST request to `url` without serializing the payload,
/// and deserialize the response body into `T`.
pub async fn post_json_text_body<T>(client: &Client, url: &str, payload: String) -> LinkResult<T>
where
    T: DeserializeOwned,
{
    let response = client
        .post(url)
        .body(payload)
        .send()
        .await
        .map_err(LinkError::from)?;

    _handle_json_response(response).await
}

async fn _handle_json_response<T: DeserializeOwned>(response: Response) -> LinkResult<T> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(LinkError::UpstreamStatus {
            status: status.as_u16(),
            body,
        });
    }

    response
        .json::<T>()
        .await
        .map_err(|e| LinkError::ParseError(e.to_string()))
}
