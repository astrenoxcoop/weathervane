use anyhow::{anyhow, Result};
use moka::{future::Cache, Expiry};
use std::time::{Duration, Instant};

use crate::{
    did_plc::plc_query,
    did_web::web_query,
    resolve::{parse_input, resolve_subject, InputType},
};

struct VerifyWorkExpiry;
struct ResolveHandleExpiry;
struct DidDocumentExpiry;

impl Expiry<String, VerifyResult> for VerifyWorkExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &VerifyResult,
        _current_time: Instant,
    ) -> Option<Duration> {
        match value {
            VerifyResult::Found => Some(Duration::from_secs(60 * 10)),
            VerifyResult::NotFound => Some(Duration::from_secs(60 * 60)),
        }
    }
}

impl Expiry<String, ResolveHandleResult> for ResolveHandleExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &ResolveHandleResult,
        _current_time: Instant,
    ) -> Option<Duration> {
        match value {
            ResolveHandleResult::Found(_) => Some(Duration::from_secs(60 * 5)),
            ResolveHandleResult::NotFound(_) => Some(Duration::from_secs(60 * 120)),
        }
    }
}

impl Expiry<String, DidDocumentResult> for DidDocumentExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &DidDocumentResult,
        _current_time: Instant,
    ) -> Option<Duration> {
        match value {
            DidDocumentResult::Found(_, _) => Some(Duration::from_secs(60 * 5)),
            DidDocumentResult::NotFound(_) => Some(Duration::from_secs(60 * 120)),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum VerifyResult {
    Found,
    NotFound,
}

#[derive(Clone, PartialEq, Eq)]
pub enum ResolveHandleResult {
    Found(String),
    NotFound(String),
}

#[derive(Clone, PartialEq, Eq)]
pub enum DidDocumentResult {
    Found(String, Vec<String>),
    NotFound(String),
}

pub(crate) fn new_worker_cache() -> Cache<String, VerifyResult> {
    let expiry = VerifyWorkExpiry;
    Cache::builder()
        .max_capacity(1024 * 20)
        .expire_after(expiry)
        .build()
}

pub fn new_resolve_handle_cache() -> Cache<String, ResolveHandleResult> {
    let expiry = ResolveHandleExpiry;
    Cache::builder()
        .max_capacity(1024 * 20)
        .expire_after(expiry)
        .build()
}

pub fn new_did_document_cache() -> Cache<String, DidDocumentResult> {
    let expiry = DidDocumentExpiry;
    Cache::builder()
        .max_capacity(1024 * 20)
        .expire_after(expiry)
        .build()
}

pub(crate) async fn resolve_subject_cached(
    cache: Cache<String, ResolveHandleResult>,
    http_client: &reqwest::Client,
    subject: &str,
) -> Result<String> {
    let cache_key = cityhasher::hash::<u64>(subject).to_string();
    if let Some(resolve_handle_result) = cache.get(&cache_key).await {
        return match resolve_handle_result {
            ResolveHandleResult::Found(did) => Ok(did),
            ResolveHandleResult::NotFound(err) => Err(anyhow!(err)),
        };
    }
    let resolved_did = resolve_subject(http_client, subject).await;

    let cache_value = match resolved_did.as_ref() {
        Ok(did) => ResolveHandleResult::Found(did.clone()),
        Err(err) => ResolveHandleResult::NotFound(err.to_string()),
    };

    cache.insert(cache_key, cache_value).await;
    resolved_did
}

pub(crate) async fn did_document_cached(
    cache: Cache<String, DidDocumentResult>,
    http_client: &reqwest::Client,
    plc_hostname: &str,
    did: &str,
) -> Result<(String, Vec<String>)> {
    let parsed_did = parse_input(did);

    if parsed_did.is_err() {
        return Err(anyhow!("Invalid DID"));
    } else if let Ok(InputType::Handle(_)) = parsed_did {
        return Err(anyhow!("Invalid DID"));
    }

    let cache_key = cityhasher::hash::<u64>(did).to_string();
    if let Some(resolve_handle_result) = cache.get(&cache_key).await {
        return match resolve_handle_result {
            DidDocumentResult::Found(did, identities) => Ok((did, identities)),
            DidDocumentResult::NotFound(err) => Err(anyhow!(err)),
        };
    }

    let query_results = match parse_input(did) {
        Ok(InputType::Plc(did)) => plc_query(http_client, plc_hostname, &did).await,
        Ok(InputType::Web(did)) => web_query(http_client, &did).await,
        _ => unreachable!(),
    };

    let cache_value = match query_results.as_ref() {
        Ok((did, identities)) => DidDocumentResult::Found(did.clone(), identities.clone()),
        Err(err) => DidDocumentResult::NotFound(err.to_string()),
    };

    cache.insert(cache_key, cache_value).await;

    query_results
}
