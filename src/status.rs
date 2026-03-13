use crate::config::{MacosAuth, SignConfig};

#[derive(Debug)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug)]
pub struct StatusReport {
    pub checks: Vec<CheckResult>,
}

impl StatusReport {
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

/// Run all status checks for the given config.
pub fn check_status(config: &SignConfig) -> StatusReport {
    let mut checks = Vec::new();

    if let Some(macos) = &config.macos {
        match macos.auth {
            MacosAuth::ApiKey => {
                check_env(&mut checks, macos.env.certificate.as_ref(), "certificate");
                check_env(
                    &mut checks,
                    macos.env.certificate_password.as_ref(),
                    "certificate-password",
                );
                check_env(
                    &mut checks,
                    macos.env.notarization_key.as_ref(),
                    "notarization-key",
                );
                check_env(
                    &mut checks,
                    macos.env.notarization_key_id.as_ref(),
                    "notarization-key-id",
                );
                check_env(
                    &mut checks,
                    macos.env.notarization_issuer.as_ref(),
                    "notarization-issuer",
                );
            }
            MacosAuth::AppleId => {
                check_env(&mut checks, macos.env.apple_id.as_ref(), "apple-id");
                check_env(&mut checks, macos.env.team_id.as_ref(), "team-id");
                check_env(&mut checks, macos.env.app_password.as_ref(), "app-password");
            }
        }

        check_tool(&mut checks, "codesign");
        check_tool(&mut checks, "xcrun");
        check_tool(&mut checks, "hdiutil");
    }

    if let Some(windows) = &config.windows {
        check_env(&mut checks, windows.env.tenant_id.as_ref(), "tenant-id");
        check_env(&mut checks, windows.env.client_id.as_ref(), "client-id");
        check_env(
            &mut checks,
            windows.env.client_secret.as_ref(),
            "client-secret",
        );
        check_env(&mut checks, windows.env.endpoint.as_ref(), "endpoint");
        check_env(
            &mut checks,
            windows.env.account_name.as_ref(),
            "account-name",
        );
        check_env(
            &mut checks,
            windows.env.cert_profile.as_ref(),
            "cert-profile",
        );
    }

    if let Some(linux) = &config.linux {
        check_env(&mut checks, linux.env.key.as_ref(), "key");
    }

    if let Some(update) = &config.update {
        check_env(&mut checks, update.env.signing_key.as_ref(), "signing-key");
    }

    StatusReport { checks }
}

fn check_env(checks: &mut Vec<CheckResult>, env_name: Option<&String>, field_name: &str) {
    let Some(env_var) = env_name else {
        checks.push(CheckResult {
            name: format!("env:{field_name}"),
            passed: false,
            detail: format!("{field_name} not configured in sign.toml"),
        });
        return;
    };

    match std::env::var(env_var) {
        Ok(val) if !val.is_empty() => {
            checks.push(CheckResult {
                name: format!("env:{env_var}"),
                passed: true,
                detail: "set".to_string(),
            });
        }
        _ => {
            checks.push(CheckResult {
                name: format!("env:{env_var}"),
                passed: false,
                detail: format!("{env_var}: not set"),
            });
        }
    }
}

fn check_tool(checks: &mut Vec<CheckResult>, tool: &str) {
    let result = crate::subprocess::run("which", &[tool], false);
    match result {
        Ok(output) if output.success => {
            checks.push(CheckResult {
                name: format!("tool:{tool}"),
                passed: true,
                detail: output.stdout.trim().to_string(),
            });
        }
        _ => {
            checks.push(CheckResult {
                name: format!("tool:{tool}"),
                passed: false,
                detail: format!("{tool}: not found in PATH"),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    #[test]
    fn status_empty_config_passes() {
        let config = SignConfig::default();
        let report = check_status(&config);
        assert!(report.checks.is_empty());
        assert!(report.all_passed());
    }

    #[test]
    fn status_checks_macos_api_key_env_vars() {
        let config = SignConfig {
            macos: Some(MacosConfig {
                identity: None,
                entitlements: None,
                auth: MacosAuth::ApiKey,
                env: MacosEnvConfig {
                    certificate: Some("NONEXISTENT_TEST_VAR_1".to_string()),
                    certificate_password: Some("NONEXISTENT_TEST_VAR_2".to_string()),
                    notarization_key: Some("NONEXISTENT_TEST_VAR_3".to_string()),
                    notarization_key_id: Some("NONEXISTENT_TEST_VAR_4".to_string()),
                    notarization_issuer: Some("NONEXISTENT_TEST_VAR_5".to_string()),
                    ..Default::default()
                },
                dmg: None,
            }),
            ..Default::default()
        };
        let report = check_status(&config);
        // 5 env var checks + 3 tool checks (codesign, xcrun, hdiutil)
        assert_eq!(report.checks.len(), 8);
        // env vars should all fail (not set)
        let env_checks: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.name.starts_with("env:"))
            .collect();
        assert_eq!(env_checks.len(), 5);
        assert!(env_checks.iter().all(|c| !c.passed));
    }

    #[test]
    fn status_checks_macos_apple_id_env_vars() {
        let config = SignConfig {
            macos: Some(MacosConfig {
                identity: None,
                entitlements: None,
                auth: MacosAuth::AppleId,
                env: MacosEnvConfig {
                    apple_id: Some("NONEXISTENT_TEST_VAR_A".to_string()),
                    team_id: Some("NONEXISTENT_TEST_VAR_B".to_string()),
                    app_password: Some("NONEXISTENT_TEST_VAR_C".to_string()),
                    ..Default::default()
                },
                dmg: None,
            }),
            ..Default::default()
        };
        let report = check_status(&config);
        // 3 env var checks + 3 tool checks
        let env_checks: Vec<_> = report
            .checks
            .iter()
            .filter(|c| c.name.starts_with("env:"))
            .collect();
        assert_eq!(env_checks.len(), 3);
    }

    #[test]
    fn status_reports_set_env_var() {
        std::env::set_var("CARGO_SIGN_TEST_KEY", "some_value");
        let config = SignConfig {
            update: Some(UpdateConfig {
                public_key: None,
                env: UpdateEnvConfig {
                    signing_key: Some("CARGO_SIGN_TEST_KEY".to_string()),
                },
            }),
            ..Default::default()
        };
        let report = check_status(&config);
        assert_eq!(report.checks.len(), 1);
        assert!(report.checks[0].passed);
        std::env::remove_var("CARGO_SIGN_TEST_KEY");
    }
}
