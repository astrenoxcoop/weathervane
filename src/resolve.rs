use anyhow::{anyhow, Result};
use futures_util::future::join;
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    AsyncResolver,
};
use std::collections::HashSet;
use std::time::Duration;

pub(crate) enum InputType {
    Handle(String),
    Plc(String),
    Web(String),
}

pub async fn resolve_handle_dns(lookup_dns: &str) -> Result<String> {
    let resolver = AsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    let lookup = resolver.txt_lookup(lookup_dns).await?;

    let dids = lookup
        .iter()
        .filter_map(|record| {
            record
                .to_string()
                .strip_prefix("did=")
                .map(|did| did.to_string())
        })
        .collect::<HashSet<String>>();

    if dids.len() > 1 {
        return Err(anyhow!("Multiple records found: {}", lookup_dns));
    }

    dids.iter()
        .next()
        .cloned()
        .ok_or(anyhow!("No records found: {}", lookup_dns))
}

pub async fn resolve_handle_http(http_client: &reqwest::Client, handle: &str) -> Result<String> {
    let lookup_url = format!("https://{}/.well-known/atproto-did", handle);

    http_client
        .get(lookup_url.clone())
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .text()
        .await
        .map_err(|err| err.into())
        .and_then(|body| {
            if body.starts_with("did:") {
                Ok(body.to_string())
            } else {
                Err(anyhow!("Invalid response from {}", lookup_url))
            }
        })
}

pub(crate) fn parse_input(input: &str) -> Result<InputType> {
    let trimmed = {
        if let Some(value) = input.strip_prefix("at://") {
            value
        } else if let Some(value) = input.strip_prefix('@') {
            value
        } else {
            input
        }
    };
    if trimmed.is_empty() {
        return Err(anyhow!("Invalid handle or DID"));
    }
    if trimmed.starts_with("did:web:") {
        Ok(InputType::Web(trimmed.to_string()))
    } else if trimmed.starts_with("did:plc:") {
        Ok(InputType::Plc(trimmed.to_string()))
    } else {
        Ok(InputType::Handle(trimmed.to_string()))
    }
}

pub async fn resolve_handle(http_client: &reqwest::Client, handle: &str) -> Result<String> {
    let trimmed = {
        if let Some(value) = handle.strip_prefix("at://") {
            value
        } else if let Some(value) = handle.strip_prefix('@') {
            value
        } else {
            handle
        }
    };

    let (dns_lookup, http_lookup) = join(
        resolve_handle_dns(&format!("_atproto.{}", trimmed)),
        resolve_handle_http(http_client, trimmed),
    )
    .await;

    match (dns_lookup, http_lookup) {
        (Ok(dns_did), Err(_)) => Ok(dns_did),
        (Err(_), Ok(http_did)) => Ok(http_did),
        (Ok(dns_did), Ok(http_did)) => {
            if dns_did == http_did {
                Ok(dns_did)
            } else {
                Err(anyhow!(
                    "DNS and HTTP lookups do not match for handle {}",
                    handle
                ))
            }
        }
        _ => Err(anyhow!("Failed to resolve handle {}", handle)),
    }
}

pub async fn resolve_subject(http_client: &reqwest::Client, subject: &str) -> Result<String> {
    match parse_input(subject)? {
        InputType::Handle(handle) => resolve_handle(http_client, &handle).await,
        InputType::Plc(did) | InputType::Web(did) => Ok(did),
    }
}
