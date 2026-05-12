mod folders;
mod page_scaling;

use axum::Router;

pub fn router() -> Router {
    Router::new().nest("/folders", folders::router())
}
