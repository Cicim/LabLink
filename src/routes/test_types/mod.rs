mod test_acc;
mod test_carp_lp;
mod test_reti;

mod utils;

use axum::Router;

pub fn router() -> Router {
    Router::new()
        .nest("/tests/ACC.TP", test_acc::router())
        .nest("/tests/RETI.TD", test_reti::router())
        .nest("/tests/CARP.LP", test_carp_lp::router())
}
