use axum::extract::FromRef;
use axum_template::engine::Engine;
use std::{ops::Deref, sync::Arc};

#[cfg(feature = "reload")]
use minijinja_autoreload::AutoReloader;

#[cfg(feature = "reload")]
pub type AppEngine = Engine<AutoReloader>;

#[cfg(feature = "embed")]
use minijinja::Environment;

use crate::worker::QueueWork;

#[cfg(feature = "embed")]
pub type AppEngine = Engine<Environment<'static>>;

pub struct InnerWebContext {
    pub(crate) external_base: String,
    pub(crate) engine: AppEngine,
    pub(crate) http_client: reqwest::Client,
    pub(crate) verify_work_tx: tokio::sync::mpsc::Sender<QueueWork>,
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
    ) -> Self {
        Self(Arc::new(InnerWebContext {
            external_base: external_base.to_string(),
            engine,
            http_client: http_client.clone(),
            verify_work_tx,
        }))
    }
}
