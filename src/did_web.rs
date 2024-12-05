use anyhow::{anyhow, Result};

use crate::did::ResolveDid;

pub(crate) async fn web_query(
    http_client: &reqwest::Client,
    did: &str,
) -> Result<(String, Vec<String>)> {
    let parts = did
        .strip_prefix("did:web:")
        .map(|trimmed| trimmed.split(":").collect::<Vec<&str>>());

    if parts.is_none() {
        return Err(anyhow!("Invalid DID"));
    }
    let mut parts = parts.unwrap();

    let hostname = parts.pop();
    if hostname.is_none() {
        return Err(anyhow!("Invalid DID"));
    }
    let hostname = hostname.unwrap();

    let url = if parts.is_empty() {
        format!("https://{}/.well-known/did.json", hostname)
    } else {
        format!("https://{}/{}/did.json", hostname, parts.join("/"))
    };

    let resolved_did: ResolveDid = http_client.get(url).send().await?.json().await?;

    Ok((
        resolved_did.id,
        resolved_did
            .also_known_as
            .iter()
            .take(25)
            .cloned()
            .collect(),
    ))
}

pub(crate) async fn web_query_simple(
    http_client: &reqwest::Client,
    hostname: &str,
) -> Result<String> {
    let url = format!("https://{}/.well-known/did.json", hostname);
    let resolved_did: ResolveDid = http_client.get(url).send().await?.json().await?;
    Ok(resolved_did.id)
}
