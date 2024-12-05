use ordermap::OrderSet;

use crate::did::is_valid_hostname;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum IdentityType {
    DIDMethodPLC(String),
    DIDMethodWeb(String),
    Handle(String),
    Domain(String),
    GitHub(String),
    Website(String),
    Unsupported(String),
}

impl IdentityType {
    pub(crate) fn pending_string(&self) -> String {
        match self {
            IdentityType::DIDMethodPLC(value) => format!("{} [DID-PLC]", value),
            IdentityType::DIDMethodWeb(value) => format!("{} [DID-WEB]", value),
            IdentityType::Handle(value) => format!("{} [Handle]", value),
            IdentityType::Domain(value) => format!("{} [DNS]", value),
            IdentityType::GitHub(value) => format!("{} [GitHub]", value),
            IdentityType::Website(value) => format!("{} [WWW]", value),
            IdentityType::Unsupported(value) => format!("{} [Unknown]", value),
        }
    }

    pub(crate) fn to_key(&self) -> String {
        match self {
            IdentityType::DIDMethodPLC(value)
            | IdentityType::DIDMethodWeb(value)
            | IdentityType::Handle(value)
            | IdentityType::Domain(value)
            | IdentityType::GitHub(value)
            | IdentityType::Website(value)
            | IdentityType::Unsupported(value) => cityhasher::hash::<u64>(value).to_string(),
        }
    }
}

pub(crate) fn parse_identities(values: &[String]) -> OrderSet<IdentityType> {
    values.iter().map(|value| parse_identity(value)).collect()
}

pub(crate) fn parse_identity(value: &str) -> IdentityType {
    if value.starts_with("did:plc:") {
        IdentityType::DIDMethodPLC(value.to_string())
    } else if value.starts_with("did:web:") {
        IdentityType::DIDMethodWeb(value.to_string())
    } else if value.starts_with("at://") {
        IdentityType::Handle(value.to_string())
    } else if value.strip_prefix("dns:").is_some_and(is_valid_hostname) {
        IdentityType::Domain(value.to_string())
    } else if value.starts_with("https://github.com/") {
        let first = value.strip_prefix("https://github.com/").map(|trimmed| {
            if let Some((first, _)) = trimmed.split_once("/") {
                first
            } else {
                trimmed
            }
        });
        if let Some(inner_value) = first {
            IdentityType::GitHub(inner_value.to_string())
        } else {
            IdentityType::Unsupported(value.to_string())
        }
    } else if value.starts_with("https://") || value.starts_with("http://") {
        IdentityType::Website(value.to_string())
    } else {
        IdentityType::Unsupported(value.to_string())
    }
}
