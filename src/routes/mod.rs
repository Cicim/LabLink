mod test_types;

use axum::Router;
use reqwest::Client;

/// Assembles all route groups into a single `Router`.
/// The shared `Client` is injected as axum `State`.
pub fn router(client: Client) -> Router {
    Router::new().nest("/api", test_types::router(client))
}
