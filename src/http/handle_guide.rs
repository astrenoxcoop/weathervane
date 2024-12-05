use anyhow::Result;
use axum::{extract::State, response::IntoResponse};
use axum_template::RenderHtml;
use minijinja::context as template_context;

use crate::{errors::WeatherVaneError, http::context::WebContext};

pub async fn handle_guide(
    State(web_context): State<WebContext>,
) -> Result<impl IntoResponse, WeatherVaneError> {
    Ok(RenderHtml(
        "guide.en-us.html",
        web_context.engine.clone(),
        template_context! {},
    )
    .into_response())
}
