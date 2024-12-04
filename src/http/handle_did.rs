use anyhow::{anyhow, Result};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use axum_htmx::HxRequest;
use axum_template::RenderHtml;
use minijinja::context as template_context;

use crate::{
    did_plc::plc_query,
    did_web::web_query,
    errors::WeatherVaneError,
    http::{context::WebContext, view_identity::IdentityView},
    identity::parse_identities,
    resolve::{parse_input, InputType},
};

pub(crate) async fn handle_did(
    State(web_context): State<WebContext>,
    HxRequest(hx_request): HxRequest,
    Path(did_slug): Path<String>,
) -> Result<impl IntoResponse, WeatherVaneError> {
    let query_results = match parse_input(&did_slug) {
        Ok(InputType::Plc(did)) => plc_query(&web_context.http_client, "plc.directory", &did).await,
        Ok(InputType::Web(did)) => web_query(&web_context.http_client, &did).await,
        Err(err) => Err(err),
        _ => Err(anyhow!("Invalid DID")),
    };

    if let Err(err) = query_results {
        return Ok(RenderHtml(
            "error.en-us.html",
            web_context.engine.clone(),
            template_context! {
                message => err.to_string(),
            },
        )
        .into_response());
    }
    let (did, identities) = query_results.unwrap();

    let parsed_identities = parse_identities(&identities);

    let identity_views: Vec<IdentityView> = parsed_identities
        .iter()
        .map(|identity| IdentityView {
            key: identity.to_key(),
            value: identity.pending_string(),
        })
        .collect();

    let template = match hx_request {
        true => "partial_did.en-us.html",
        false => "did.en-us.html",
    };

    Ok(RenderHtml(
        template,
        web_context.engine.clone(),
        template_context! {
            did,
            identities => identity_views,
        },
    )
    .into_response())
}
