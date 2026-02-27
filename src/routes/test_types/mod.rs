mod test_acc;

use axum::Router;
use reqwest::Client;

pub fn router(client: Client) -> Router {
    Router::new().nest("/tests", test_acc::router(client))
}
