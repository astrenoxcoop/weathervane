use anyhow::Result;
use std::env;
use tokio::net::TcpListener;
use tokio::signal;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use tracing_subscriber::prelude::*;
use weathervane::{
    cache::{new_did_document_cache, new_resolve_handle_cache},
    http::{
        context::{AppEngine, WebContext},
        server::build_router,
    },
    worker::{QueueWork, VerifyTask},
};

#[cfg(feature = "embed")]
use weathervane::http::templates::embed_env;

#[cfg(feature = "reload")]
use weathervane::http::templates::reload_env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "weathervane=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let version = weathervane::config::version()?;

    env::args().for_each(|arg| {
        if arg == "--version" {
            println!("{}", version);
            std::process::exit(0);
        }
    });

    let config = weathervane::config::Config::new()?;

    let mut client_builder = reqwest::Client::builder();
    for ca_certificate in config.certificate_bundles.as_ref() {
        tracing::info!("Loading CA certificate: {:?}", ca_certificate);
        let cert = std::fs::read(ca_certificate)?;
        let cert = reqwest::Certificate::from_pem(&cert)?;
        client_builder = client_builder.add_root_certificate(cert);
    }

    client_builder = client_builder.user_agent(config.user_agent.clone());
    let http_client = client_builder.build()?;

    #[cfg(feature = "embed")]
    let jinja = embed_env::build_env(config.external_base.clone(), config.version.clone());

    #[cfg(feature = "reload")]
    let jinja = reload_env::build_env(&config.external_base, &config.version);

    let (verify_work_tx, mut verify_work_rx) = tokio::sync::mpsc::channel::<QueueWork>(100);

    let resolve_handle_cache = new_resolve_handle_cache();
    let did_document_cache = new_did_document_cache();

    let web_context = WebContext::new(
        config.external_base.as_str(),
        AppEngine::from(jinja),
        &http_client,
        verify_work_tx,
        resolve_handle_cache,
        did_document_cache,
        config.plc_hostname.clone(),
    );

    let app = build_router(web_context.clone());

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    {
        let tracker = tracker.clone();
        let inner_token = token.clone();

        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        tokio::spawn(async move {
            tokio::select! {
                () = inner_token.cancelled() => { },
                _ = terminate => {},
                _ = ctrl_c => {},
            }

            tracker.close();
            inner_token.cancel();
        });
    }

    {
        let task = VerifyTask::new(&http_client, config.plc_hostname.clone(), token.clone());
        let inner_token = token.clone();
        tracker.spawn(async move {
            let _ = task.run_background(&mut verify_work_rx).await;
            inner_token.cancel();
        });
    }

    {
        let inner_config = config.clone();
        let http_port = *inner_config.http_port.as_ref();
        let inner_token = token.clone();
        tracker.spawn(async move {
            let listener = TcpListener::bind(&format!("0.0.0.0:{}", http_port))
                .await
                .unwrap();

            let shutdown_token = inner_token.clone();
            let result = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    tokio::select! {
                        () = shutdown_token.cancelled() => { }
                    }
                    tracing::info!("axum graceful shutdown complete");
                })
                .await;
            if let Err(err) = result {
                tracing::error!("axum task failed: {}", err);
            }

            inner_token.cancel();
        });
    }

    tracker.wait().await;

    Ok(())
}
