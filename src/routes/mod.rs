mod test_types;

use axum::{routing::get, Json, Router};
use reqwest::Client;
use serde::Serialize;

use crate::errors::LinkResult;

#[derive(Serialize)]
struct VersionInfo {
    name: &'static str,
    version: &'static str,
}

async fn root_handler() -> LinkResult<Json<VersionInfo>> {
    Ok(Json(VersionInfo {
        name: "LabLink",
        version: "0.1.0",
    }))
}

/// Assembles all route groups into a single `Router`.
/// The shared `Client` is injected as axum `State`.
pub fn router(client: Client) -> Router {
    Router::new()
        .route("/", get(root_handler))
        .nest("/api", test_types::router(client))
}
