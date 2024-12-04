use std::time::Duration;

use axum::{
    http::HeaderValue,
    routing::{get, post},
    Router,
};
use axum_htmx::AutoVaryLayer;
use http::{
    header::{ACCEPT, ACCEPT_LANGUAGE},
    Method,
};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::http::{
    context::WebContext, handle_did::handle_did, handle_did_stream::handle_did_stream,
    handle_index::handle_index, handle_validate::handle_validate,
};

pub fn build_router(web_context: WebContext) -> Router {
    let serve_dir = ServeDir::new("static");

    Router::new()
        .route("/", get(handle_index))
        .route("/validate", post(handle_validate))
        .route("/did/:did", get(handle_did))
        .route("/did/:did/updates", get(handle_did_stream))
        .nest_service("/static", serve_dir.clone())
        .fallback_service(serve_dir)
        .layer((
            TraceLayer::new_for_http(),
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(web_context.external_base.parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET])
                .allow_headers([ACCEPT_LANGUAGE, ACCEPT]),
        )
        .layer(AutoVaryLayer)
        .with_state(web_context.clone())
}
