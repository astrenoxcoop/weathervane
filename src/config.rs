use anyhow::{anyhow, Result};

#[derive(Clone)]
pub struct HttpPort(u16);

#[derive(Clone)]
pub struct CertificateBundles(Vec<String>);

#[derive(Clone)]
pub struct Config {
    pub version: String,
    pub http_port: HttpPort,
    pub external_base: String,
    pub certificate_bundles: CertificateBundles,
    pub user_agent: String,
    pub plc_hostname: String,
}

impl Config {
    pub fn new() -> Result<Self> {
        let http_port: HttpPort = default_env("HTTP_PORT", "4060").try_into()?;
        let external_base = require_env("EXTERNAL_BASE")?;

        let certificate_bundles: CertificateBundles =
            optional_env("CERTIFICATE_BUNDLES").try_into()?;

        let default_user_agent = format!(
            "weathervane ({}; +https://github.com/astrenoxcoop/weathervane)",
            version()?
        );

        let user_agent = default_env("USER_AGENT", &default_user_agent);

        let plc_hostname = default_env("PLC_HOSTNAME", "plc.directory");

        Ok(Self {
            version: version()?,
            http_port,
            external_base,
            certificate_bundles,
            user_agent,
            plc_hostname,
        })
    }
}

fn require_env(name: &str) -> Result<String> {
    std::env::var(name)
        .map_err(|err| anyhow::Error::new(err).context(anyhow!("{} must be set", name)))
}

fn optional_env(name: &str) -> String {
    std::env::var(name).unwrap_or("".to_string())
}

fn default_env(name: &str, default_value: &str) -> String {
    std::env::var(name).unwrap_or(default_value.to_string())
}

pub fn version() -> Result<String> {
    option_env!("GIT_HASH")
        .or(option_env!("CARGO_PKG_VERSION"))
        .map(|val| val.to_string())
        .ok_or(anyhow!("one of GIT_HASH or CARGO_PKG_VERSION must be set"))
}

impl TryFrom<String> for HttpPort {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Ok(Self(80))
        } else {
            value.parse::<u16>().map(Self).map_err(|err| {
                anyhow::Error::new(err).context(anyhow!("parsing PORT into u16 failed"))
            })
        }
    }
}

impl AsRef<u16> for HttpPort {
    fn as_ref(&self) -> &u16 {
        &self.0
    }
}

impl TryFrom<String> for CertificateBundles {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .split(';')
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
                .collect::<Vec<String>>(),
        ))
    }
}

impl AsRef<Vec<String>> for CertificateBundles {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}
