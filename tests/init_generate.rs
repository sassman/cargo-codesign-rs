use cargo_codesign::config::{LinuxMethod, MacosAuth};
use cargo_codesign::init::{generate_sign_toml, InitSelections};

#[test]
fn generate_macos_apple_id_only() {
    let selections = InitSelections {
        macos: true,
        macos_auth: Some(MacosAuth::AppleId),
        windows: false,
        linux: false,
        linux_method: None,
        update: false,
    };
    let toml_str = generate_sign_toml(&selections);
    assert!(toml_str.contains("[macos]"));
    assert!(toml_str.contains("auth = \"apple-id\""));
    assert!(toml_str.contains("[macos.env]"));
    assert!(toml_str.contains("apple-id = \"APPLE_ID\""));
    assert!(toml_str.contains("team-id = \"APPLE_TEAM_ID\""));
    assert!(toml_str.contains("app-password = \"APPLE_APP_PASSWORD\""));
    assert!(!toml_str.contains("[windows]"));
    assert!(!toml_str.contains("[linux]"));
    assert!(!toml_str.contains("[update]"));
}

#[test]
fn generate_macos_api_key_only() {
    let selections = InitSelections {
        macos: true,
        macos_auth: Some(MacosAuth::ApiKey),
        windows: false,
        linux: false,
        linux_method: None,
        update: false,
    };
    let toml_str = generate_sign_toml(&selections);
    assert!(toml_str.contains("auth = \"api-key\""));
    assert!(toml_str.contains("certificate = \"APPLE_CERTIFICATE\""));
    assert!(toml_str.contains("notarization-key = \"APPLE_NOTARIZATION_KEY\""));
}

#[test]
fn generate_windows_only() {
    let selections = InitSelections {
        macos: false,
        macos_auth: None,
        windows: true,
        linux: false,
        linux_method: None,
        update: false,
    };
    let toml_str = generate_sign_toml(&selections);
    assert!(toml_str.contains("[windows]"));
    assert!(toml_str.contains("timestamp-server = \"http://timestamp.acs.microsoft.com\""));
    assert!(toml_str.contains("[windows.env]"));
    assert!(toml_str.contains("tenant-id = \"AZURE_TENANT_ID\""));
    assert!(!toml_str.contains("[macos]"));
}

#[test]
fn generate_linux_cosign() {
    let selections = InitSelections {
        macos: false,
        macos_auth: None,
        windows: false,
        linux: true,
        linux_method: Some(LinuxMethod::Cosign),
        update: false,
    };
    let toml_str = generate_sign_toml(&selections);
    assert!(toml_str.contains("[linux]"));
    assert!(toml_str.contains("method = \"cosign\""));
    assert!(toml_str.contains("key = \"COSIGN_PRIVATE_KEY\""));
}

#[test]
fn generate_linux_minisign() {
    let selections = InitSelections {
        macos: false,
        macos_auth: None,
        windows: false,
        linux: true,
        linux_method: Some(LinuxMethod::Minisign),
        update: false,
    };
    let toml_str = generate_sign_toml(&selections);
    assert!(toml_str.contains("method = \"minisign\""));
    assert!(toml_str.contains("key = \"MINISIGN_PRIVATE_KEY\""));
}

#[test]
fn generate_update_only() {
    let selections = InitSelections {
        macos: false,
        macos_auth: None,
        windows: false,
        linux: false,
        linux_method: None,
        update: true,
    };
    let toml_str = generate_sign_toml(&selections);
    assert!(toml_str.contains("[update]"));
    assert!(toml_str.contains("signing-key = \"UPDATE_SIGNING_KEY\""));
}

#[test]
fn generate_all_platforms() {
    let selections = InitSelections {
        macos: true,
        macos_auth: Some(MacosAuth::AppleId),
        windows: true,
        linux: true,
        linux_method: Some(LinuxMethod::Cosign),
        update: true,
    };
    let toml_str = generate_sign_toml(&selections);
    assert!(toml_str.contains("[macos]"));
    assert!(toml_str.contains("[windows]"));
    assert!(toml_str.contains("[linux]"));
    assert!(toml_str.contains("[update]"));
}

#[test]
fn generated_toml_is_valid() {
    let selections = InitSelections {
        macos: true,
        macos_auth: Some(MacosAuth::ApiKey),
        windows: true,
        linux: true,
        linux_method: Some(LinuxMethod::Cosign),
        update: true,
    };
    let toml_str = generate_sign_toml(&selections);
    let parsed: Result<cargo_codesign::config::SignConfig, _> = toml::from_str(&toml_str);
    assert!(parsed.is_ok(), "Generated TOML failed to parse: {toml_str}");
}
