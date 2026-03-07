#[test]
fn parse_minimal_macos_api_key_config() {
    let toml_str = r#"
[macos]
identity = "Developer ID Application"
entitlements = "entitlements.plist"
auth = "api-key"

[macos.env]
certificate = "MACOS_CERTIFICATE"
certificate-password = "MACOS_CERTIFICATE_PASSWORD"
notarization-key = "APPLE_NOTARIZATION_KEY"
notarization-key-id = "APPLE_NOTARIZATION_KEY_ID"
notarization-issuer = "APPLE_NOTARIZATION_ISSUER_ID"
"#;

    let config: cargo_sign::config::SignConfig = toml::from_str(toml_str).unwrap();
    let macos = config.macos.unwrap();
    assert_eq!(macos.identity, Some("Developer ID Application".to_string()));
    assert_eq!(macos.auth, cargo_sign::config::MacosAuth::ApiKey);
    assert_eq!(macos.env.certificate, Some("MACOS_CERTIFICATE".to_string()));
}

#[test]
fn parse_minimal_macos_apple_id_config() {
    let toml_str = r#"
[macos]
identity = "Developer ID Application"
auth = "apple-id"

[macos.env]
apple-id = "APPLE_ID"
team-id = "APPLE_TEAM_ID"
app-password = "APPLE_APP_PASSWORD"
"#;

    let config: cargo_sign::config::SignConfig = toml::from_str(toml_str).unwrap();
    let macos = config.macos.unwrap();
    assert_eq!(macos.auth, cargo_sign::config::MacosAuth::AppleId);
    assert_eq!(macos.env.apple_id, Some("APPLE_ID".to_string()));
}

#[test]
fn parse_full_config_all_platforms() {
    let toml_str = r#"
[macos]
identity = "Developer ID Application"
entitlements = "entitlements.plist"
auth = "api-key"

[macos.env]
certificate = "MACOS_CERTIFICATE"
certificate-password = "MACOS_CERTIFICATE_PASSWORD"
notarization-key = "APPLE_NOTARIZATION_KEY"
notarization-key-id = "APPLE_NOTARIZATION_KEY_ID"
notarization-issuer = "APPLE_NOTARIZATION_ISSUER_ID"

[windows]
timestamp-server = "http://timestamp.acs.microsoft.com"

[windows.env]
tenant-id = "AZURE_TENANT_ID"
client-id = "AZURE_CLIENT_ID"
client-secret = "AZURE_CLIENT_SECRET"
endpoint = "AZURE_SIGNING_ENDPOINT"
account-name = "AZURE_SIGNING_ACCOUNT_NAME"
cert-profile = "AZURE_SIGNING_CERT_PROFILE"

[linux]
method = "cosign"

[linux.env]
key = "COSIGN_PRIVATE_KEY"

[update]
public-key = "update-signing.pub"

[update.env]
signing-key = "UPDATE_SIGNING_KEY"

[status]
cert-warn-days = 60
cert-error-days = 7
"#;

    let config: cargo_sign::config::SignConfig = toml::from_str(toml_str).unwrap();
    assert!(config.macos.is_some());
    assert!(config.windows.is_some());
    assert!(config.linux.is_some());
    assert!(config.update.is_some());
    let status = config.status.unwrap();
    assert_eq!(status.cert_warn_days, Some(60));
    assert_eq!(status.cert_error_days, Some(7));
}

#[test]
fn parse_empty_config() {
    let toml_str = "";
    let config: cargo_sign::config::SignConfig = toml::from_str(toml_str).unwrap();
    assert!(config.macos.is_none());
    assert!(config.windows.is_none());
    assert!(config.linux.is_none());
    assert!(config.update.is_none());
    assert!(config.status.is_none());
}
