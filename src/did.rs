use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolveDid {
    pub(crate) id: String,
    pub(crate) also_known_as: Vec<String>,
}

pub fn is_valid_hostname(hostname: &str) -> bool {
    fn is_valid_char(byte: u8) -> bool {
        byte.is_ascii_lowercase()
            || byte.is_ascii_uppercase()
            || byte.is_ascii_digit()
            || byte == b'-'
            || byte == b'.'
    }
    !(hostname.ends_with(".localhost")
        || hostname.ends_with(".internal")
        || hostname.ends_with(".arpa")
        || hostname.ends_with(".local")
        || hostname.bytes().any(|byte| !is_valid_char(byte))
        || hostname.split('.').any(|label| {
            label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-')
        })
        || hostname.is_empty()
        || hostname.len() > 253)
}

pub fn is_valid_handle(handle: &str) -> Option<String> {
    let trimmed = {
        if let Some(value) = handle.strip_prefix("at://") {
            value
        } else if let Some(value) = handle.strip_prefix('@') {
            value
        } else {
            handle
        }
    };
    if is_valid_hostname(trimmed) && trimmed.chars().any(|c| c == '.') {
        Some(trimmed.to_string())
    } else {
        None
    }
}
