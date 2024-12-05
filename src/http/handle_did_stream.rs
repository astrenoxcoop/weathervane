use std::{convert::Infallible, time::Duration};

use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
};
use axum_template::RenderHtml;
use axum_template::TemplateEngine;
use futures_util::stream::Stream;
use minijinja::context as template_context;
use tokio::sync::mpsc;

use crate::{
    cache::did_document_cached,
    errors::WeatherVaneError,
    http::context::{AppEngine, WebContext},
    identity::parse_identities,
    worker::{QueueWork, VerifyWork},
};

pub(crate) async fn handle_did_stream(
    State(web_context): State<WebContext>,
    Path(did_slug): Path<String>,
) -> Result<impl IntoResponse, WeatherVaneError> {
    let query_results = did_document_cached(
        web_context.did_document_cache.clone(),
        &web_context.http_client,
        &web_context.plc_hostname,
        &did_slug,
    )
    .await;

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

    let (tx, rx) = mpsc::channel::<VerifyWork>(parsed_identities.len() + 1);

    web_context
        .verify_work_tx
        .send(QueueWork {
            did,
            tx,
            identities: Vec::from_iter(parsed_identities),
        })
        .await?;

    let sse_resp = stream_maker(web_context.engine.clone(), rx).await;

    Ok(sse_resp.into_response())
}

async fn stream_maker(
    engine: AppEngine,
    rx: mpsc::Receiver<VerifyWork>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = rx;
    let stream = async_stream::stream! {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        while let Some(res) = rx.recv().await {

            if res == VerifyWork::Done() {
                break;
            }

            let (key, context) = match res {
                VerifyWork::Ok(identity) => {
                    (format!("identity-{}", identity.to_key()), template_context! { identity => identity.pending_string(), success => true })
                },
                VerifyWork::Error(identity, message) => {
                    (format!("identity-{}", identity.to_key()), template_context! { identity => identity.pending_string(), success => false, message => message })
                },
                _ => unreachable!(),
            };

            let render_result = engine.render("partial_key.en-us.html", context);
            match render_result {
                Ok(rendered) => {
                    let event =  Event::default().event(key).data(rendered);
                    yield Ok(event);
                },
                Err(err) => {
                    let event =  Event::default().event(key).data(err.to_string());
                    yield Ok(event);
                }
            }

        }

        rx.close();

        // Nick: The "done" event and payload is used to indicate that no further information will
        // be sent. Clients should disconnect and not reconnect. For clients that misbehave, return
        // an infinite loop.
        loop {
            interval.tick().await;
            yield Ok(Event::default().event("done").data("done"));
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}
