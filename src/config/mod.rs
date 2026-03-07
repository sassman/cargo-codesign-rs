pub mod resolve;

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SignConfig {
    pub macos: Option<MacosConfig>,
    pub windows: Option<WindowsConfig>,
    pub linux: Option<LinuxConfig>,
    pub update: Option<UpdateConfig>,
    pub status: Option<StatusConfig>,
}

// --- macOS ---

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MacosAuth {
    ApiKey,
    AppleId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacosConfig {
    pub identity: Option<String>,
    pub entitlements: Option<PathBuf>,
    pub auth: MacosAuth,
    pub env: MacosEnvConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct MacosEnvConfig {
    // api-key mode
    pub certificate: Option<String>,
    pub certificate_password: Option<String>,
    pub notarization_key: Option<String>,
    pub notarization_key_id: Option<String>,
    pub notarization_issuer: Option<String>,
    // apple-id mode
    pub apple_id: Option<String>,
    pub team_id: Option<String>,
    pub app_password: Option<String>,
}

// --- Windows ---

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct WindowsConfig {
    pub timestamp_server: Option<String>,
    pub env: WindowsEnvConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct WindowsEnvConfig {
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub endpoint: Option<String>,
    pub account_name: Option<String>,
    pub cert_profile: Option<String>,
}

// --- Linux ---

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LinuxMethod {
    Cosign,
    Minisign,
    Gpg,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinuxConfig {
    pub method: LinuxMethod,
    pub env: LinuxEnvConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LinuxEnvConfig {
    pub key: Option<String>,
}

// --- Update signing ---

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UpdateConfig {
    pub public_key: Option<PathBuf>,
    pub env: UpdateEnvConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UpdateEnvConfig {
    pub signing_key: Option<String>,
}

// --- Status ---

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct StatusConfig {
    pub cert_warn_days: Option<u32>,
    pub cert_error_days: Option<u32>,
}
