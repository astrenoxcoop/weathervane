use anyhow::Result;

use crate::did::ResolveDid;

pub(crate) async fn plc_query(
    http_client: &reqwest::Client,
    plc_hostname: &str,
    did: &str,
) -> Result<(String, Vec<String>)> {
    let url = format!("https://{}/{}", plc_hostname, did);

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
