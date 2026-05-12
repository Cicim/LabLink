mod folders;

use axum::Router;

pub fn router() -> Router {
    Router::new().nest("/folders", folders::router())
}
