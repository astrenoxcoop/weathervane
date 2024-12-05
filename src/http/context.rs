use axum::extract::FromRef;
use axum_template::engine::Engine;
use moka::future::Cache;
use std::{ops::Deref, sync::Arc};

#[cfg(feature = "reload")]
use minijinja_autoreload::AutoReloader;

#[cfg(feature = "reload")]
pub type AppEngine = Engine<AutoReloader>;

#[cfg(feature = "embed")]
use minijinja::Environment;

use crate::{
    cache::{DidDocumentResult, ResolveHandleResult},
    worker::QueueWork,
};

#[cfg(feature = "embed")]
pub type AppEngine = Engine<Environment<'static>>;

pub struct InnerWebContext {
    pub(crate) external_base: String,
    pub(crate) engine: AppEngine,
    pub(crate) http_client: reqwest::Client,
    pub(crate) verify_work_tx: tokio::sync::mpsc::Sender<QueueWork>,
    pub(crate) resolve_handle_cache: Cache<String, ResolveHandleResult>,
    pub(crate) did_document_cache: Cache<String, DidDocumentResult>,
    pub(crate) plc_hostname: String,
}

#[derive(Clone, FromRef)]
pub struct WebContext(pub(crate) Arc<InnerWebContext>);

impl Deref for WebContext {
    type Target = InnerWebContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl WebContext {
    pub fn new(
        external_base: &str,
        engine: AppEngine,
        http_client: &reqwest::Client,
        verify_work_tx: tokio::sync::mpsc::Sender<QueueWork>,
        resolve_handle_cache: Cache<String, ResolveHandleResult>,
        did_document_cache: Cache<String, DidDocumentResult>,
        plc_hostname: String,
    ) -> Self {
        Self(Arc::new(InnerWebContext {
            external_base: external_base.to_string(),
            engine,
            http_client: http_client.clone(),
            verify_work_tx,
            resolve_handle_cache,
            did_document_cache,
            plc_hostname,
        }))
    }
}
