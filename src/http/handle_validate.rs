use anyhow::Result;
use axum::{extract::State, response::IntoResponse, Form};
use axum_htmx::{HxRedirect, HxRequest};
use axum_template::RenderHtml;
use http::StatusCode;
use minijinja::context as template_context;

use crate::{cache::resolve_subject_cached, errors::WeatherVaneError, http::context::WebContext};

#[derive(serde::Deserialize)]
pub(crate) struct ValidateForm {
    subject: String,
}

pub(crate) async fn handle_validate(
    State(web_context): State<WebContext>,
    HxRequest(hx_request): HxRequest,
    Form(web_form): Form<ValidateForm>,
) -> Result<impl IntoResponse, WeatherVaneError> {
    if !hx_request {
        return Ok(RenderHtml(
            "error.en-us.html",
            web_context.engine.clone(),
            template_context! {
                message => "Invalid Request",
            },
        )
        .into_response());
    }

    let resolved_did = resolve_subject_cached(
        web_context.resolve_handle_cache.clone(),
        &web_context.http_client,
        &web_form.subject,
    )
    .await;
    if let Err(err) = resolved_did {
        return Ok(RenderHtml(
            "partial_validate.en-us.html",
            web_context.engine.clone(),
            template_context! {
                subject_value => web_form.subject,
                subject_error => err.to_string(),
            },
        )
        .into_response());
    }
    let resolved_did = resolved_did.unwrap();

    let hx_redirect = HxRedirect::try_from(format!("/did/{}", resolved_did).as_str());
    if let Err(err) = hx_redirect {
        tracing::error!("Failed to create HxLocation: {}", err);
        return Ok(RenderHtml(
            "error.en-us.html",
            web_context.engine.clone(),
            template_context! { message => "Internal Server Error" },
        )
        .into_response());
    }
    let hx_redirect = hx_redirect.unwrap();
    Ok((StatusCode::OK, hx_redirect, "").into_response())
}
