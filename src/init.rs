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
