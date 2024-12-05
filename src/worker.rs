use anyhow::Result;
use moka::future::Cache;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;

use crate::{
    cache::{new_worker_cache, VerifyResult},
    identity::IdentityType,
};

pub struct QueueWork {
    pub(crate) did: String,
    pub(crate) tx: tokio::sync::mpsc::Sender<VerifyWork>,
    pub(crate) identities: Vec<IdentityType>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum VerifyWork {
    Ok(IdentityType),
    Error(IdentityType, String),
    Done(),
}

pub struct VerifyTask {
    pub(crate) http_client: reqwest::Client,
    pub(crate) plc_hostname: String,
    pub(crate) cancellation_token: CancellationToken,

    cache: Cache<String, VerifyResult>,
}

impl VerifyTask {
    pub fn new(
        http_client: &reqwest::Client,
        plc_hostname: String,
        cancellation_token: CancellationToken,
    ) -> Self {
        let cache = new_worker_cache();
        Self {
            http_client: http_client.clone(),
            cancellation_token,
            plc_hostname,
            cache,
        }
    }

    pub async fn run_background(&self, rx: &mut Receiver<QueueWork>) -> Result<()> {
        tracing::debug!("VerifyTask started");

        loop {
            tokio::select! {
            () = self.cancellation_token.cancelled() => {
                break;
            },
                r = rx.recv() => {
                    match r {
                        Some(work) => {
                            if let Err(err) = self.process_work(&work).await {
                                tracing::error!("DomainImporterTask task failed: {}", err);
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }
            }
        }

        tracing::debug!("VerifyTask started");

        Ok(())
    }

    async fn process_work(&self, work: &QueueWork) -> Result<()> {
        let mut identity_queue = work.identities.clone();
        while let Some(identity) = identity_queue.pop() {
            let cache_key = format!("{}-{}", work.did, identity.to_key());

            if let Some(verify_result) = self.cache.get(&cache_key).await {
                let message = match verify_result {
                    VerifyResult::Found => VerifyWork::Ok(identity),
                    VerifyResult::NotFound => {
                        VerifyWork::Error(identity, "DID not found".to_string())
                    }
                };
                if let Err(err) = work.tx.send(message).await {
                    tracing::error!(error = ?err, "Failed to write to work channel.");
                }
                continue;
            }

            let verify_result = match identity.clone() {
                IdentityType::Handle(identity_value) => {
                    handle::validate(&self.http_client, &work.did, &identity_value).await
                }
                IdentityType::GitHub(identity_value) => {
                    github::validate(&self.http_client, &work.did, &identity_value).await
                }
                IdentityType::Domain(identity_value) => {
                    domain::validate(&work.did, &identity_value).await
                }
                IdentityType::Website(identity_value) => {
                    website::validate(&self.http_client, &work.did, &identity_value).await
                }
                IdentityType::DIDMethodPLC(identity_value) => {
                    did_method_plc::validate(
                        &self.http_client,
                        &self.plc_hostname,
                        &work.did,
                        &identity_value,
                    )
                    .await
                }
                IdentityType::DIDMethodWeb(identity_value) => {
                    did_method_web::validate(&self.http_client, &work.did, &identity_value).await
                }
                _ => VerifyResult::NotFound,
            };

            self.cache.insert(cache_key, verify_result.clone()).await;
            let message = match verify_result {
                VerifyResult::Found => VerifyWork::Ok(identity),
                VerifyResult::NotFound => VerifyWork::Error(identity, "DID not found".to_string()),
            };
            if let Err(err) = work.tx.send(message).await {
                tracing::error!(error = ?err, "Failed to write to work channel.");
            }
        }
        if let Err(err) = work.tx.send(VerifyWork::Done()).await {
            tracing::error!(error = ?err, "Failed to write to work channel.");
        }

        Ok(())
    }
}

pub(crate) mod domain {
    use crate::resolve::resolve_handle_dns;

    use super::VerifyResult;

    pub(crate) async fn validate(did: &str, identity_value: &str) -> VerifyResult {
        tracing::info!(handle = identity_value, did = did, "processing domain");
        let trimmed = identity_value
            .strip_prefix("dns:")
            .unwrap_or(identity_value);
        if let Ok(found_did) = resolve_handle_dns(trimmed).await {
            tracing::info!(did = found_did, "did resolved");
            if found_did == did {
                return VerifyResult::Found;
            }
        }

        VerifyResult::NotFound
    }
}

pub(crate) mod did_method_plc {
    use crate::did_plc::plc_query;

    use super::VerifyResult;

    pub(crate) async fn validate(
        http_client: &reqwest::Client,
        plc_hostname: &str,
        did: &str,
        identity_value: &str,
    ) -> VerifyResult {
        tracing::info!(
            handle = identity_value,
            did = did,
            "processing did-method-plc"
        );

        let plc_results = plc_query(http_client, plc_hostname, identity_value).await;
        if plc_results.is_err() {
            return VerifyResult::NotFound;
        }
        let (_, values) = plc_results.unwrap();

        if values.iter().any(|x| x == did) {
            return VerifyResult::Found;
        }

        VerifyResult::NotFound
    }
}

pub(crate) mod did_method_web {
    use crate::did_web::web_query;

    use super::VerifyResult;

    pub(crate) async fn validate(
        http_client: &reqwest::Client,
        did: &str,
        identity_value: &str,
    ) -> VerifyResult {
        tracing::info!(
            handle = identity_value,
            did = did,
            "processing did-method-web"
        );

        let web_results = web_query(http_client, identity_value).await;
        if web_results.is_err() {
            return VerifyResult::NotFound;
        }
        let (_, values) = web_results.unwrap();

        if values.iter().any(|x| x == did) {
            return VerifyResult::Found;
        }

        VerifyResult::NotFound
    }
}

pub(crate) mod handle {
    use crate::resolve::resolve_handle;

    use super::VerifyResult;

    pub(crate) async fn validate(
        http_client: &reqwest::Client,
        did: &str,
        identity_value: &str,
    ) -> VerifyResult {
        tracing::info!(handle = identity_value, did = did, "processing handle");
        if let Ok(found_did) = resolve_handle(http_client, identity_value).await {
            tracing::info!(did = found_did, "did resolved");
            if found_did == did {
                return VerifyResult::Found;
            }
        }

        VerifyResult::NotFound
    }
}

pub(crate) mod github {
    use serde::Deserialize;

    use super::{handle, VerifyResult};

    pub(crate) async fn validate(
        http_client: &reqwest::Client,
        did: &str,
        identity_value: &str,
    ) -> VerifyResult {
        let response = http_client
            .get(format!(
                "https://api.github.com/users/{}/social_accounts",
                identity_value
            ))
            .send()
            .await;
        if response.is_err() {
            return VerifyResult::NotFound;
        }
        let response = response.unwrap();

        let social_accounts: Result<Vec<GitHubSocial>, _> = response.json().await;
        if social_accounts.is_err() {
            return VerifyResult::NotFound;
        }
        let social_accounts = social_accounts.unwrap();

        let bsky_handle = social_accounts
            .iter()
            .find(|x| x.provider == "bluesky")
            .and_then(|social_profile| {
                social_profile
                    .url
                    .strip_prefix("https://bsky.app/profile/")
                    .map(|trimmed| {
                        if let Some((first, _)) = trimmed.split_once("/") {
                            first
                        } else {
                            trimmed
                        }
                    })
            });
        if bsky_handle.is_none() {
            return VerifyResult::NotFound;
        }
        let bsky_handle = bsky_handle.unwrap();

        handle::validate(http_client, did, bsky_handle).await
    }

    #[derive(Deserialize)]
    struct GitHubSocial {
        provider: String,
        url: String,
    }
}

pub(crate) mod website {
    use scraper::{Html, Selector};
    use std::str::FromStr;
    use std::time::Duration;
    use url::Url;

    use super::VerifyResult;

    pub(crate) async fn validate(
        http_client: &reqwest::Client,
        did: &str,
        identity_value: &str,
    ) -> VerifyResult {
        let url = Url::from_str(identity_value);
        if url.is_err() {
            return VerifyResult::NotFound;
        }
        let url = url.unwrap();
        if url.scheme() != "http" && url.scheme() != "https" {
            return VerifyResult::NotFound;
        }
        if url.host().is_none() {
            return VerifyResult::NotFound;
        }
        if !url.username().is_empty() || url.password().is_some() {
            return VerifyResult::NotFound;
        }
        if url.path().is_empty() {
            return VerifyResult::NotFound;
        }
        if url.query().is_some() {
            return VerifyResult::NotFound;
        }

        let response = http_client
            .get(url.to_string())
            .timeout(Duration::from_secs(3))
            .send()
            .await;
        if response.is_err() {
            return VerifyResult::NotFound;
        }
        let response = response.unwrap();

        let body = response.text().await;
        if body.is_err() {
            return VerifyResult::NotFound;
        }
        let body = body.unwrap();

        let document = Html::parse_document(&body);

        let selectors = vec![
            Selector::parse(r#"link[rel~="did"]"#).unwrap(),
            Selector::parse(r#"link[rel~="me"]"#).unwrap(),
        ];
        for selector in selectors {
            for element in document.select(&selector) {
                if let Some(found_did) = element.value().attr("href") {
                    if found_did == did {
                        return VerifyResult::Found;
                    }
                }
            }
        }

        VerifyResult::NotFound
    }
}
