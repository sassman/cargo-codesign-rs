use crate::config::{LinuxMethod, MacosAuth};
use std::fmt::Write;

#[allow(clippy::struct_excessive_bools)]
pub struct InitSelections {
    pub macos: bool,
    pub macos_auth: Option<MacosAuth>,
    pub windows: bool,
    pub linux: bool,
    pub linux_method: Option<LinuxMethod>,
    pub update: bool,
}

pub fn generate_sign_toml(selections: &InitSelections) -> String {
    let mut out = String::new();

    if selections.macos {
        let auth = selections
            .macos_auth
            .as_ref()
            .unwrap_or(&MacosAuth::AppleId);
        writeln!(out, "[macos]").unwrap();
        writeln!(out, "identity = \"Developer ID Application\"").unwrap();
        writeln!(out, "entitlements = \"entitlements.plist\"").unwrap();
        match auth {
            MacosAuth::ApiKey => {
                writeln!(out, "auth = \"api-key\"").unwrap();
                writeln!(out).unwrap();
                writeln!(out, "[macos.env]").unwrap();
                writeln!(out, "certificate = \"MACOS_CERTIFICATE\"").unwrap();
                writeln!(out, "certificate-password = \"MACOS_CERTIFICATE_PASSWORD\"").unwrap();
                writeln!(out, "notarization-key = \"APPLE_NOTARIZATION_KEY\"").unwrap();
                writeln!(out, "notarization-key-id = \"APPLE_NOTARIZATION_KEY_ID\"").unwrap();
                writeln!(
                    out,
                    "notarization-issuer = \"APPLE_NOTARIZATION_ISSUER_ID\""
                )
                .unwrap();
            }
            MacosAuth::AppleId => {
                writeln!(out, "auth = \"apple-id\"").unwrap();
                writeln!(out).unwrap();
                writeln!(out, "[macos.env]").unwrap();
                writeln!(out, "apple-id = \"APPLE_ID\"").unwrap();
                writeln!(out, "team-id = \"APPLE_TEAM_ID\"").unwrap();
                writeln!(out, "app-password = \"APPLE_APP_PASSWORD\"").unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    if selections.windows {
        writeln!(out, "[windows]").unwrap();
        writeln!(
            out,
            "timestamp-server = \"http://timestamp.acs.microsoft.com\""
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(out, "[windows.env]").unwrap();
        writeln!(out, "tenant-id = \"AZURE_TENANT_ID\"").unwrap();
        writeln!(out, "client-id = \"AZURE_CLIENT_ID\"").unwrap();
        writeln!(out, "client-secret = \"AZURE_CLIENT_SECRET\"").unwrap();
        writeln!(out, "endpoint = \"AZURE_SIGNING_ENDPOINT\"").unwrap();
        writeln!(out, "account-name = \"AZURE_SIGNING_ACCOUNT_NAME\"").unwrap();
        writeln!(out, "cert-profile = \"AZURE_SIGNING_CERT_PROFILE\"").unwrap();
        writeln!(out).unwrap();
    }

    if selections.linux {
        let method = selections
            .linux_method
            .as_ref()
            .unwrap_or(&LinuxMethod::Cosign);
        writeln!(out, "[linux]").unwrap();
        match method {
            LinuxMethod::Cosign => {
                writeln!(out, "method = \"cosign\"").unwrap();
                writeln!(out).unwrap();
                writeln!(out, "[linux.env]").unwrap();
                writeln!(out, "key = \"COSIGN_PRIVATE_KEY\"").unwrap();
            }
            LinuxMethod::Minisign => {
                writeln!(out, "method = \"minisign\"").unwrap();
                writeln!(out).unwrap();
                writeln!(out, "[linux.env]").unwrap();
                writeln!(out, "key = \"MINISIGN_PRIVATE_KEY\"").unwrap();
            }
            LinuxMethod::Gpg => {
                writeln!(out, "method = \"gpg\"").unwrap();
                writeln!(out).unwrap();
                writeln!(out, "[linux.env]").unwrap();
                writeln!(out, "key = \"GPG_PRIVATE_KEY\"").unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    if selections.update {
        writeln!(out, "[update]").unwrap();
        writeln!(out, "public-key = \"update-signing.pub\"").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "[update.env]").unwrap();
        writeln!(out, "signing-key = \"UPDATE_SIGNING_KEY\"").unwrap();
        writeln!(out).unwrap();
    }

    out.trim_end().to_string() + "\n"
}

const BOOK_BASE_URL: &str = "https://sassman.github.io/cargo-codesign-rs";

pub struct CredentialCheck {
    pub env_var: String,
    pub description: String,
    pub is_set: bool,
    pub help_url: String,
}

pub fn check_credentials(selections: &InitSelections) -> Vec<CredentialCheck> {
    let mut checks = Vec::new();

    if selections.macos {
        let auth = selections
            .macos_auth
            .as_ref()
            .unwrap_or(&MacosAuth::AppleId);
        check_macos_creds(&mut checks, auth);
    }

    if selections.windows {
        check_windows_creds(&mut checks);
    }

    if selections.linux {
        let method = selections.linux_method.unwrap_or(LinuxMethod::Cosign);
        check_linux_creds(&mut checks, method);
    }

    if selections.update {
        check_cred(
            &mut checks,
            "UPDATE_SIGNING_KEY",
            "ed25519 private key for update signing (run `cargo codesign keygen`)",
            "update-signing/keygen.html",
        );
    }

    checks
}

fn check_macos_creds(checks: &mut Vec<CredentialCheck>, auth: &MacosAuth) {
    match auth {
        MacosAuth::ApiKey => {
            check_cred(
                checks,
                "MACOS_CERTIFICATE",
                "base64-encoded .p12 Developer ID certificate",
                "macos/credentials.html",
            );
            check_cred(
                checks,
                "MACOS_CERTIFICATE_PASSWORD",
                "password for the .p12 certificate",
                "macos/credentials.html",
            );
            check_cred(
                checks,
                "APPLE_NOTARIZATION_KEY",
                "base64-encoded App Store Connect API key (.p8)",
                "macos/auth-modes.html",
            );
            check_cred(
                checks,
                "APPLE_NOTARIZATION_KEY_ID",
                "API key ID from App Store Connect > Keys",
                "macos/auth-modes.html",
            );
            check_cred(
                checks,
                "APPLE_NOTARIZATION_ISSUER_ID",
                "Issuer ID from App Store Connect > Keys",
                "macos/auth-modes.html",
            );
        }
        MacosAuth::AppleId => {
            check_cred(
                checks,
                "APPLE_ID",
                "your Apple ID email address",
                "macos/credentials.html",
            );
            check_cred(
                checks,
                "APPLE_TEAM_ID",
                "Team ID from App Store Connect > Membership",
                "macos/credentials.html",
            );
            check_cred(
                checks,
                "APPLE_APP_PASSWORD",
                "app-specific password for notarization",
                "macos/auth-modes.html",
            );
        }
    }
}

fn check_windows_creds(checks: &mut Vec<CredentialCheck>) {
    check_cred(
        checks,
        "AZURE_TENANT_ID",
        "Azure AD tenant ID",
        "windows/credentials.html",
    );
    check_cred(
        checks,
        "AZURE_CLIENT_ID",
        "Azure AD application (client) ID",
        "windows/credentials.html",
    );
    check_cred(
        checks,
        "AZURE_CLIENT_SECRET",
        "Azure AD client secret",
        "windows/credentials.html",
    );
    check_cred(
        checks,
        "AZURE_SIGNING_ENDPOINT",
        "Azure Trusted Signing endpoint URL",
        "windows/credentials.html",
    );
    check_cred(
        checks,
        "AZURE_SIGNING_ACCOUNT_NAME",
        "Trusted Signing account name",
        "windows/credentials.html",
    );
    check_cred(
        checks,
        "AZURE_SIGNING_CERT_PROFILE",
        "certificate profile name",
        "windows/credentials.html",
    );
}

fn check_linux_creds(checks: &mut Vec<CredentialCheck>, method: LinuxMethod) {
    match method {
        LinuxMethod::Cosign => {
            check_cred(
                checks,
                "COSIGN_PRIVATE_KEY",
                "cosign private key (or use keyless OIDC in CI)",
                "linux/credentials.html",
            );
        }
        LinuxMethod::Minisign => {
            check_cred(
                checks,
                "MINISIGN_PRIVATE_KEY",
                "minisign private key",
                "linux/credentials.html",
            );
        }
        LinuxMethod::Gpg => {
            check_cred(
                checks,
                "GPG_PRIVATE_KEY",
                "GPG private key (armor-encoded)",
                "linux/credentials.html",
            );
        }
    }
}

fn check_cred(checks: &mut Vec<CredentialCheck>, env_var: &str, description: &str, path: &str) {
    let is_set = std::env::var(env_var).is_ok_and(|v| !v.is_empty());
    checks.push(CredentialCheck {
        env_var: env_var.to_string(),
        description: description.to_string(),
        is_set,
        help_url: format!("{BOOK_BASE_URL}/{path}"),
    });
}

pub fn print_credential_report(checks: &[CredentialCheck]) {
    for check in checks {
        if check.is_set {
            eprintln!("  \u{2713} {:<35} set", check.env_var);
        } else {
            eprintln!("  \u{2717} {:<35} {}", check.env_var, check.description);
            eprintln!("    \u{2192} {}", check.help_url);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_check_detects_set_env_var() {
        std::env::set_var("CARGO_CODESIGN_TEST_INIT_1", "value");
        let mut checks = Vec::new();
        check_cred(
            &mut checks,
            "CARGO_CODESIGN_TEST_INIT_1",
            "test",
            "test.html",
        );
        assert!(checks[0].is_set);
        std::env::remove_var("CARGO_CODESIGN_TEST_INIT_1");
    }

    #[test]
    fn credential_check_detects_missing_env_var() {
        std::env::remove_var("CARGO_CODESIGN_NONEXISTENT_VAR_XYZ");
        let mut checks = Vec::new();
        check_cred(
            &mut checks,
            "CARGO_CODESIGN_NONEXISTENT_VAR_XYZ",
            "test",
            "test.html",
        );
        assert!(!checks[0].is_set);
    }

    #[test]
    fn credential_check_macos_apple_id_returns_three() {
        let selections = InitSelections {
            macos: true,
            macos_auth: Some(MacosAuth::AppleId),
            windows: false,
            linux: false,
            linux_method: None,
            update: false,
        };
        let checks = check_credentials(&selections);
        assert_eq!(checks.len(), 3);
    }

    #[test]
    fn credential_check_macos_api_key_returns_five() {
        let selections = InitSelections {
            macos: true,
            macos_auth: Some(MacosAuth::ApiKey),
            windows: false,
            linux: false,
            linux_method: None,
            update: false,
        };
        let checks = check_credentials(&selections);
        assert_eq!(checks.len(), 5);
    }

    #[test]
    fn credential_check_windows_returns_six() {
        let selections = InitSelections {
            macos: false,
            macos_auth: None,
            windows: true,
            linux: false,
            linux_method: None,
            update: false,
        };
        let checks = check_credentials(&selections);
        assert_eq!(checks.len(), 6);
    }

    #[test]
    fn help_url_contains_book_base() {
        let selections = InitSelections {
            macos: true,
            macos_auth: Some(MacosAuth::AppleId),
            windows: false,
            linux: false,
            linux_method: None,
            update: false,
        };
        let checks = check_credentials(&selections);
        assert!(checks.iter().all(|c| c.help_url.starts_with(BOOK_BASE_URL)));
    }
}
