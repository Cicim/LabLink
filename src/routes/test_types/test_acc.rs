use axum::{extract::State, routing::get, Json, Router};
use reqwest::Client;

use crate::errors::LinkResult;
use crate::utils::steel_extractor::{get_test_results, TestResult};

#[derive(Clone)]
pub struct LinkState {
    pub client: Client,
}

/// /api/tests/ACC route
async fn test_handler(State(state): State<LinkState>) -> LinkResult<Json<Vec<TestResult>>> {
    let test_results = get_test_results(&state.client, "VA 3200/2025").await?;

    Ok(Json(test_results))
}

pub fn router(client: Client) -> Router {
    let state = LinkState { client };
    Router::new()
        .route("/ACC", get(test_handler))
        .with_state(state)
}
