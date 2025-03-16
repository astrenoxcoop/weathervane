use anyhow::Result;
use axum::{extract::{Query, State}, response::IntoResponse};
use axum_template::RenderHtml;
use minijinja::context as template_context;
use serde::{Deserialize, Serialize};

use crate::{errors::WeatherVaneError, http::context::WebContext};

#[derive(Deserialize, Serialize)]
pub struct FormHints {
    pub query: Option<String>,
}

pub async fn handle_index(
    State(web_context): State<WebContext>,
    Query(form_hints): Query<FormHints>,
) -> Result<impl IntoResponse, WeatherVaneError> {
    Ok(RenderHtml(
        "index.en-us.html",
        web_context.engine.clone(),
        template_context! {
            subject_value => form_hints.query,
        },
    )
    .into_response())
}
