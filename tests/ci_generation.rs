use cargo_codesign::ci::generate_workflow;
use cargo_codesign::config::*;

fn macos_apple_id_config() -> SignConfig {
    SignConfig {
        macos: Some(MacosConfig {
            identity: Some("Developer ID Application".to_string()),
            entitlements: Some("entitlements.plist".into()),
            auth: MacosAuth::AppleId,
            env: MacosEnvConfig {
                apple_id: Some("APPLE_ID".to_string()),
                team_id: Some("APPLE_TEAM_ID".to_string()),
                app_password: Some("APPLE_APP_PASSWORD".to_string()),
                ..Default::default()
            },
            dmg: None,
        }),
        ..Default::default()
    }
}

fn windows_config() -> SignConfig {
    SignConfig {
        windows: Some(WindowsConfig {
            timestamp_server: Some("http://timestamp.acs.microsoft.com".to_string()),
            env: WindowsEnvConfig {
                tenant_id: Some("AZURE_TENANT_ID".to_string()),
                client_id: Some("AZURE_CLIENT_ID".to_string()),
                client_secret: Some("AZURE_CLIENT_SECRET".to_string()),
                endpoint: Some("AZURE_SIGNING_ENDPOINT".to_string()),
                account_name: Some("AZURE_SIGNING_ACCOUNT_NAME".to_string()),
                cert_profile: Some("AZURE_SIGNING_CERT_PROFILE".to_string()),
            },
        }),
        ..Default::default()
    }
}

#[test]
fn generates_macos_workflow() {
    let config = macos_apple_id_config();
    let yaml = generate_workflow(&config);
    assert!(yaml.contains("sign-macos:"));
    assert!(yaml.contains("runs-on: macos-latest"));
    assert!(yaml.contains("APPLE_ID: ${{ secrets.APPLE_ID }}"));
    assert!(yaml.contains("APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}"));
    assert!(yaml.contains("APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}"));
    assert!(!yaml.contains("sign-windows:"));
    assert!(!yaml.contains("sign-linux:"));
}

#[test]
fn generates_windows_workflow() {
    let config = windows_config();
    let yaml = generate_workflow(&config);
    assert!(yaml.contains("sign-windows:"));
    assert!(yaml.contains("runs-on: windows-latest"));
    assert!(yaml.contains("AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}"));
    assert!(!yaml.contains("sign-macos:"));
}

#[test]
fn generates_multi_platform_workflow() {
    let mut config = macos_apple_id_config();
    config.windows = windows_config().windows;
    let yaml = generate_workflow(&config);
    assert!(yaml.contains("sign-macos:"));
    assert!(yaml.contains("sign-windows:"));
}

#[test]
fn workflow_starts_with_name_and_trigger() {
    let config = macos_apple_id_config();
    let yaml = generate_workflow(&config);
    assert!(yaml.starts_with("name: Sign Release Artifacts"));
    assert!(yaml.contains("workflow_call:"));
}

#[test]
fn workflow_has_jobs_key() {
    let config = macos_apple_id_config();
    let yaml = generate_workflow(&config);
    assert!(yaml.contains("jobs:"));
}
