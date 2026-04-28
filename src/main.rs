use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Codesign(SignArgs),
}

#[derive(clap::Args)]
#[command(version, about = "Cross-platform binary signing for Rust projects")]
struct SignArgs {
    #[command(subcommand)]
    command: SignCommand,

    /// Path to sign.toml
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,

    /// Print subprocess commands and full output
    #[arg(long, global = true)]
    verbose: bool,

    /// Validate and print what would be done, without executing
    #[arg(long, global = true)]
    dry_run: bool,

    /// Machine-readable output
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum SignCommand {
    /// Validate credentials, certs, and tool availability
    Status,
    /// Sign, notarize, and staple macOS artifacts
    Macos {
        /// Path to .app bundle (triggers full sign+DMG+notarize+staple chain)
        #[arg(long, conflicts_with_all = ["ci_import_cert", "ci_cleanup_cert"])]
        app: Option<std::path::PathBuf>,
        /// Path to existing DMG (codesign + notarize + staple)
        #[arg(long, conflicts_with_all = ["ci_import_cert", "ci_cleanup_cert"])]
        dmg: Option<std::path::PathBuf>,
        /// Entitlements plist (overrides config)
        #[arg(long)]
        entitlements: Option<std::path::PathBuf>,
        /// Signing identity substring
        #[arg(long)]
        identity: Option<String>,
        /// Sign only, skip notarization
        #[arg(long)]
        skip_notarize: bool,
        /// Skip stapling
        #[arg(long)]
        skip_staple: bool,
        /// CI: import base64-encoded certificate into ephemeral keychain
        #[arg(long, conflicts_with_all = ["app", "dmg", "ci_cleanup_cert"])]
        ci_import_cert: bool,
        /// CI: delete ephemeral keychain from a previous --ci-import-cert
        #[arg(long, conflicts_with_all = ["app", "dmg", "ci_import_cert"])]
        ci_cleanup_cert: bool,
    },
    /// Sign a Windows executable via Azure Trusted Signing
    Windows {
        /// Install Azure Trusted Signing tools if missing
        #[arg(long)]
        install_tools: bool,
    },
    /// Sign a Linux artifact with cosign, minisign, or gpg
    Linux {
        /// Archive to sign
        #[arg(long)]
        archive: std::path::PathBuf,
        /// Override signing method from config (cosign, minisign, gpg)
        #[arg(long)]
        method: Option<String>,
        /// Signature output path
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Sign a release archive for in-app update verification (ed25519)
    Update {
        /// Archive to sign
        #[arg(long)]
        archive: std::path::PathBuf,
        /// Signature output path
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        /// Env var name for the signing key
        #[arg(long, default_value = "UPDATE_SIGNING_KEY")]
        key_env: String,
        /// Also verify with this public key after signing
        #[arg(long)]
        public_key: Option<std::path::PathBuf>,
    },
    /// Verify a signed artifact or signature file
    Verify {
        /// Artifact to verify
        artifact: std::path::PathBuf,
        /// Verification method: macos, windows, update, cosign, minisign, gpg
        #[arg(long)]
        method: String,
        /// Explicit signature/bundle file path (auto-detected if omitted)
        #[arg(long)]
        signature: Option<std::path::PathBuf>,
        /// Public key file for update/minisign verification
        #[arg(long)]
        public_key: Option<std::path::PathBuf>,
    },
    /// Generate an ed25519 keypair for update signing
    Keygen {
        /// Output path for private key
        #[arg(long, default_value = "./update-signing.key")]
        output_private: std::path::PathBuf,
        /// Output path for public key
        #[arg(long, default_value = "./update-signing.pub")]
        output_public: std::path::PathBuf,
    },
    /// Generate CI workflow (GitHub Actions YAML) from sign.toml
    Ci {
        /// Output path for generated workflow
        #[arg(long, default_value = ".github/workflows/release-sign.yml")]
        output: std::path::PathBuf,
    },
    /// Create sign.toml with guided prompts
    Init,
}

fn main() {
    let CargoCli::Codesign(args) = CargoCli::parse();
    match args.command {
        SignCommand::Status => cmd_status(args.config.as_deref()),
        SignCommand::Macos {
            app,
            dmg,
            entitlements,
            identity,
            skip_notarize,
            skip_staple,
            ci_import_cert,
            ci_cleanup_cert,
        } => {
            if ci_import_cert {
                cmd_macos_ci_import(args.config.as_deref(), args.verbose);
            } else if ci_cleanup_cert {
                cmd_macos_ci_cleanup(args.verbose);
            } else {
                cmd_macos(
                    args.config.as_deref(),
                    app.as_deref(),
                    dmg.as_deref(),
                    entitlements.as_deref(),
                    identity.as_deref(),
                    skip_notarize,
                    skip_staple,
                    args.verbose,
                );
            }
        }
        SignCommand::Windows { install_tools } => {
            cmd_windows(args.config.as_deref(), install_tools, args.verbose);
        }
        SignCommand::Linux {
            archive,
            method,
            output,
        } => {
            cmd_linux(
                args.config.as_deref(),
                &archive,
                method.as_deref(),
                output.as_deref(),
                args.verbose,
            );
        }
        SignCommand::Update {
            archive,
            output,
            key_env,
            public_key,
        } => cmd_update(&archive, output.as_deref(), &key_env, public_key.as_deref()),
        SignCommand::Verify {
            artifact,
            method,
            signature,
            public_key,
        } => cmd_verify(
            &artifact,
            &method,
            signature.as_deref(),
            public_key.as_deref(),
            args.verbose,
        ),
        SignCommand::Keygen {
            output_private,
            output_public,
        } => cmd_keygen(&output_private, &output_public),
        SignCommand::Ci { output } => cmd_ci(args.config.as_deref(), &output),
        SignCommand::Init => cmd_init(),
    }
}

#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
fn cmd_macos(
    config_path: Option<&std::path::Path>,
    app: Option<&std::path::Path>,
    dmg: Option<&std::path::Path>,
    entitlements_override: Option<&std::path::Path>,
    identity_override: Option<&str>,
    skip_notarize: bool,
    skip_staple: bool,
    verbose: bool,
) {
    let _ = dotenvy::dotenv();

    let (config, _resolved_path, warnings) = if let Some(path) = config_path {
        cargo_codesign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_codesign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    };

    for w in &warnings {
        eprintln!("{w}");
    }

    let macos_config = config.macos.as_ref().unwrap_or_else(|| {
        eprintln!("✗ No [macos] section in sign.toml");
        std::process::exit(2);
    });

    let identity = identity_override
        .or(macos_config.identity.as_deref())
        .unwrap_or("Developer ID Application");

    let entitlements = entitlements_override.or(macos_config.entitlements.as_deref());

    // If a previous `--ci-import-cert` ran, the absolute path to the
    // ephemeral keychain is persisted in the state file. Resolving the
    // identity from that keychain explicitly via `--keychain <path>` avoids
    // any reliance on the user's keychain search list.
    let ci_keychain = cargo_codesign::platform::macos::load_keychain_state(
        &cargo_codesign::platform::macos::keychain_state_path(),
    );
    if let Some(ref kc) = ci_keychain {
        eprintln!("  Using CI keychain: {}", kc.display());
    }
    let ci_keychain_ref = ci_keychain.as_deref();

    if let Some(dmg_path) = dmg {
        macos_dmg_mode(
            dmg_path,
            identity,
            ci_keychain_ref,
            macos_config,
            skip_notarize,
            skip_staple,
            verbose,
        );
    } else if let Some(app_path) = app {
        macos_app_mode(
            app_path,
            identity,
            entitlements,
            ci_keychain_ref,
            macos_config,
            skip_notarize,
            skip_staple,
            verbose,
        );
    } else {
        macos_bare_binary_mode(identity, ci_keychain_ref, verbose);
    }
}

#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
fn macos_dmg_mode(
    dmg_path: &std::path::Path,
    identity: &str,
    keychain: Option<&std::path::Path>,
    macos_config: &cargo_codesign::config::MacosConfig,
    skip_notarize: bool,
    skip_staple: bool,
    verbose: bool,
) {
    use cargo_codesign::platform::macos;

    eprintln!("[1/3] Codesigning DMG...");
    let opts = macos::CodesignOpts {
        identity,
        entitlements: None,
        keychain,
        verbose,
    };
    macos::codesign_dmg(dmg_path, &opts).unwrap_or_else(|e| {
        eprintln!("✗ DMG codesigning failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ DMG signed");

    if !skip_notarize {
        eprintln!("[2/3] Notarizing DMG...");
        notarize_artifact(dmg_path, macos_config, verbose);
        eprintln!("  ✓ Notarized");
    }

    if !skip_notarize && !skip_staple {
        eprintln!("[3/3] Stapling...");
        macos::staple(dmg_path, verbose).unwrap_or_else(|e| {
            eprintln!("✗ Stapling failed: {e}");
            std::process::exit(1);
        });
        eprintln!("  ✓ Stapled");
    }

    eprintln!("✓ Done: {}", dmg_path.display());
}

#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)]
fn macos_app_mode(
    app_path: &std::path::Path,
    identity: &str,
    entitlements: Option<&std::path::Path>,
    keychain: Option<&std::path::Path>,
    macos_config: &cargo_codesign::config::MacosConfig,
    skip_notarize: bool,
    skip_staple: bool,
    verbose: bool,
) {
    use cargo_codesign::platform::macos;

    let app_name = app_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    eprintln!("[1/5] Codesigning .app bundle...");
    let opts = macos::CodesignOpts {
        identity,
        entitlements,
        keychain,
        verbose,
    };
    macos::codesign_app(app_path, &opts).unwrap_or_else(|e| {
        eprintln!("✗ App codesigning failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ App signed");

    let dmg_path = app_path.with_extension("dmg");
    eprintln!("[2/5] Creating DMG...");
    macos::create_dmg(
        app_path,
        &dmg_path,
        &app_name,
        macos_config.dmg.as_ref(),
        verbose,
    )
    .unwrap_or_else(|e| {
        eprintln!("✗ DMG creation failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ DMG created: {}", dmg_path.display());

    eprintln!("[3/5] Codesigning DMG...");
    let dmg_opts = macos::CodesignOpts {
        identity,
        entitlements: None,
        keychain,
        verbose,
    };
    macos::codesign_dmg(&dmg_path, &dmg_opts).unwrap_or_else(|e| {
        eprintln!("✗ DMG codesigning failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ DMG signed");

    if !skip_notarize {
        eprintln!("[4/5] Notarizing DMG...");
        notarize_artifact(&dmg_path, macos_config, verbose);
        eprintln!("  ✓ Notarized");
    }

    if !skip_notarize && !skip_staple {
        eprintln!("[5/5] Stapling...");
        macos::staple(&dmg_path, verbose).unwrap_or_else(|e| {
            eprintln!("✗ Stapling failed: {e}");
            std::process::exit(1);
        });
        eprintln!("  ✓ Stapled");
    }

    eprintln!("✓ Done: {}", dmg_path.display());
}

#[cfg(target_os = "macos")]
fn macos_bare_binary_mode(identity: &str, keychain: Option<&std::path::Path>, verbose: bool) {
    use cargo_codesign::platform::macos;

    eprintln!("Discovering binaries via cargo metadata...");
    let binaries = cargo_codesign::discovery::discover_binaries().unwrap_or_else(|e| {
        eprintln!("✗ Binary discovery failed: {e}");
        std::process::exit(1);
    });

    if binaries.is_empty() {
        eprintln!("✗ No binary targets found");
        std::process::exit(1);
    }

    let opts = macos::CodesignOpts {
        identity,
        entitlements: None,
        keychain,
        verbose,
    };

    for bin in &binaries {
        let src = bin.release_path();
        if !src.exists() {
            eprintln!("  ⚠ Skipping {} (not built: {})", bin.name, src.display());
            continue;
        }
        eprintln!("  Signing {}...", bin.name);
        macos::codesign(&src, &opts).unwrap_or_else(|e| {
            eprintln!("✗ Codesigning failed for {}: {e}", bin.name);
            std::process::exit(1);
        });

        let dst = bin.signed_release_path();
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("✗ Failed to create output directory: {e}");
                std::process::exit(1);
            });
        }
        std::fs::copy(&src, &dst).unwrap_or_else(|e| {
            eprintln!("✗ Failed to copy signed binary: {e}");
            std::process::exit(1);
        });
        eprintln!("  ✓ {} → {}", bin.name, dst.display());
    }
    eprintln!("✓ Done");
}

#[cfg(target_os = "macos")]
fn cmd_macos_ci_import(config_path: Option<&std::path::Path>, verbose: bool) {
    let _ = dotenvy::dotenv();

    let (config, _resolved_path, warnings) = if let Some(path) = config_path {
        cargo_codesign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_codesign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    };
    for w in &warnings {
        eprintln!("{w}");
    }

    let macos_config = config.macos.as_ref().unwrap_or_else(|| {
        eprintln!("✗ No [macos] section in sign.toml");
        std::process::exit(2);
    });

    let cert_b64 = resolve_env(macos_config.env.certificate.as_ref(), "certificate");
    let cert_password = resolve_env(
        macos_config.env.certificate_password.as_ref(),
        "certificate-password",
    );

    let temp_dir = tempfile::TempDir::new().unwrap_or_else(|e| {
        eprintln!("✗ Failed to create temp dir: {e}");
        std::process::exit(1);
    });

    let p12_path =
        cargo_codesign::platform::macos::decode_cert_to_tempfile(&cert_b64, temp_dir.path())
            .unwrap_or_else(|e| {
                eprintln!("✗ Failed to decode certificate: {e}");
                std::process::exit(1);
            });

    let keychain_path =
        cargo_codesign::platform::macos::import_certificate(&p12_path, &cert_password, verbose)
            .unwrap_or_else(|e| {
                eprintln!("✗ Certificate import failed: {e}");
                std::process::exit(1);
            });

    let state_path = cargo_codesign::platform::macos::keychain_state_path();
    cargo_codesign::platform::macos::save_keychain_state(&state_path, &keychain_path)
        .unwrap_or_else(|e| {
            eprintln!("✗ Failed to save keychain state: {e}");
            std::process::exit(1);
        });

    eprintln!(
        "✓ Certificate imported (keychain: {})",
        keychain_path.display()
    );
}

#[cfg(not(target_os = "macos"))]
fn cmd_macos_ci_import(_config_path: Option<&std::path::Path>, _verbose: bool) {
    eprintln!("✗ --ci-import-cert requires macOS");
    std::process::exit(3);
}

#[cfg(target_os = "macos")]
fn cmd_macos_ci_cleanup(verbose: bool) {
    let state_path = cargo_codesign::platform::macos::keychain_state_path();

    let Some(keychain_path) = cargo_codesign::platform::macos::load_keychain_state(&state_path)
    else {
        eprintln!("⚠ No keychain state found — nothing to clean up");
        return;
    };

    cargo_codesign::platform::macos::delete_keychain(&keychain_path, verbose).unwrap_or_else(|e| {
        eprintln!("✗ Keychain cleanup failed: {e}");
        std::process::exit(1);
    });

    if let Err(e) = std::fs::remove_file(&state_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!(
                "⚠ Failed to remove keychain state file {}: {e}",
                state_path.display()
            );
        }
    }
    eprintln!("✓ Keychain cleaned up ({})", keychain_path.display());
}

#[cfg(not(target_os = "macos"))]
fn cmd_macos_ci_cleanup(_verbose: bool) {
    eprintln!("✗ --ci-cleanup-cert requires macOS");
    std::process::exit(3);
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn resolve_env(name: Option<&String>, field: &str) -> String {
    let env_name = name.unwrap_or_else(|| {
        eprintln!("✗ {field} not configured in sign.toml");
        std::process::exit(2);
    });
    std::env::var(env_name).unwrap_or_else(|_| {
        eprintln!("✗ Environment variable {env_name} not set (needed for {field})");
        std::process::exit(1);
    })
}

#[cfg(target_os = "macos")]
fn notarize_artifact(
    artifact: &std::path::Path,
    macos_config: &cargo_codesign::config::MacosConfig,
    verbose: bool,
) {
    use cargo_codesign::platform::macos;

    match macos_config.auth {
        cargo_codesign::config::MacosAuth::ApiKey => {
            let key_b64 = resolve_env(
                macos_config.env.notarization_key.as_ref(),
                "notarization-key",
            );
            let key_id = resolve_env(
                macos_config.env.notarization_key_id.as_ref(),
                "notarization-key-id",
            );
            let issuer = resolve_env(
                macos_config.env.notarization_issuer.as_ref(),
                "notarization-issuer",
            );

            let key_bytes =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key_b64.trim())
                    .unwrap_or_else(|e| {
                        eprintln!("✗ Failed to decode notarization key from base64: {e}");
                        std::process::exit(1);
                    });
            let key_dir = tempfile::TempDir::new().unwrap();
            let key_path = key_dir.path().join("AuthKey.p8");
            std::fs::write(&key_path, key_bytes).unwrap();

            macos::notarize_api_key(artifact, &key_path, &key_id, &issuer, verbose).unwrap_or_else(
                |e| {
                    eprintln!("✗ Notarization failed: {e}");
                    std::process::exit(1);
                },
            );
        }
        cargo_codesign::config::MacosAuth::AppleId => {
            let apple_id = resolve_env(macos_config.env.apple_id.as_ref(), "apple-id");
            let team_id = resolve_env(macos_config.env.team_id.as_ref(), "team-id");
            let password = resolve_env(macos_config.env.app_password.as_ref(), "app-password");

            macos::notarize_apple_id(artifact, &apple_id, &team_id, &password, verbose)
                .unwrap_or_else(|e| {
                    eprintln!("✗ Notarization failed: {e}");
                    std::process::exit(1);
                });
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[allow(clippy::too_many_arguments)]
fn cmd_macos(
    _config_path: Option<&std::path::Path>,
    _app: Option<&std::path::Path>,
    _dmg: Option<&std::path::Path>,
    _entitlements_override: Option<&std::path::Path>,
    _identity_override: Option<&str>,
    _skip_notarize: bool,
    _skip_staple: bool,
    _verbose: bool,
) {
    eprintln!("✗ cargo sign macos requires macOS");
    std::process::exit(3);
}

#[cfg(target_os = "windows")]
fn cmd_windows(config_path: Option<&std::path::Path>, install_tools: bool, verbose: bool) {
    use cargo_codesign::platform::windows;

    let _ = dotenvy::dotenv();

    let (config, _resolved_path, warnings) = if let Some(path) = config_path {
        cargo_codesign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_codesign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    };

    for w in &warnings {
        eprintln!("{w}");
    }

    let windows_config = config.windows.as_ref().unwrap_or_else(|| {
        eprintln!("✗ No [windows] section in sign.toml");
        std::process::exit(2);
    });

    let dlib_path = if install_tools {
        eprintln!("Installing Azure Trusted Signing tools...");
        windows::install_tools(verbose).unwrap_or_else(|e| {
            eprintln!("✗ Tool installation failed: {e}");
            std::process::exit(3);
        })
    } else {
        std::path::PathBuf::from("Azure.CodeSigning.Dlib.dll")
    };

    let endpoint = resolve_env(windows_config.env.endpoint.as_ref(), "endpoint");
    let account_name = resolve_env(windows_config.env.account_name.as_ref(), "account-name");
    let cert_profile = resolve_env(windows_config.env.cert_profile.as_ref(), "cert-profile");
    let timestamp = windows_config
        .timestamp_server
        .as_deref()
        .unwrap_or("http://timestamp.acs.microsoft.com");

    let binaries = cargo_codesign::discovery::discover_binaries().unwrap_or_else(|e| {
        eprintln!("✗ Binary discovery failed: {e}");
        std::process::exit(1);
    });

    let opts = windows::SignOpts {
        endpoint: &endpoint,
        account_name: &account_name,
        cert_profile: &cert_profile,
        timestamp_server: timestamp,
        dlib_path: &dlib_path,
        verbose,
    };

    for bin in &binaries {
        let exe_path = bin.release_path().with_extension("exe");
        if !exe_path.exists() {
            eprintln!("  ⚠ Skipping {} (not built)", bin.name);
            continue;
        }
        eprintln!("  Signing {}...", bin.name);
        windows::sign_exe(&exe_path, &opts).unwrap_or_else(|e| {
            eprintln!("✗ Signing failed for {}: {e}", bin.name);
            std::process::exit(1);
        });
        eprintln!("  ✓ {}", bin.name);
    }

    eprintln!("✓ Done");
}

#[cfg(not(target_os = "windows"))]
fn cmd_windows(_config_path: Option<&std::path::Path>, _install_tools: bool, _verbose: bool) {
    eprintln!("✗ cargo codesign windows requires Windows");
    std::process::exit(3);
}

#[cfg(target_os = "linux")]
fn cmd_linux(
    config_path: Option<&std::path::Path>,
    archive: &std::path::Path,
    method_override: Option<&str>,
    output: Option<&std::path::Path>,
    verbose: bool,
) {
    let _ = dotenvy::dotenv();

    let (config, _resolved_path, warnings) = if let Some(path) = config_path {
        cargo_codesign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_codesign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    };

    for w in &warnings {
        eprintln!("{w}");
    }

    let linux_config = config.linux.as_ref().unwrap_or_else(|| {
        eprintln!("✗ No [linux] section in sign.toml");
        std::process::exit(2);
    });

    let method = if let Some(m) = method_override {
        match m {
            "cosign" => cargo_codesign::config::LinuxMethod::Cosign,
            "minisign" => cargo_codesign::config::LinuxMethod::Minisign,
            "gpg" => cargo_codesign::config::LinuxMethod::Gpg,
            other => {
                eprintln!("✗ Unknown method: {other} (expected: cosign, minisign, gpg)");
                std::process::exit(2);
            }
        }
    } else {
        linux_config.method
    };

    let opts = cargo_codesign::platform::linux::SignOpts { verbose, output };

    let sig_path = match method {
        cargo_codesign::config::LinuxMethod::Cosign => {
            eprintln!("Signing with cosign (keyless OIDC)...");
            cargo_codesign::platform::linux::sign_cosign(archive, &opts)
        }
        cargo_codesign::config::LinuxMethod::Minisign => {
            let key_env = linux_config
                .env
                .key
                .as_deref()
                .unwrap_or("MINISIGN_PRIVATE_KEY");
            let key_content = std::env::var(key_env).unwrap_or_else(|_| {
                eprintln!("✗ Environment variable {key_env} not set");
                std::process::exit(1);
            });
            eprintln!("Signing with minisign...");
            cargo_codesign::platform::linux::sign_minisign(archive, &key_content, &opts)
        }
        cargo_codesign::config::LinuxMethod::Gpg => {
            eprintln!("Signing with gpg...");
            cargo_codesign::platform::linux::sign_gpg(archive, &opts)
        }
    };

    match sig_path {
        Ok(path) => eprintln!("✓ Signature: {}", path.display()),
        Err(e) => {
            eprintln!("✗ Signing failed: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn cmd_linux(
    _config_path: Option<&std::path::Path>,
    _archive: &std::path::Path,
    _method_override: Option<&str>,
    _output: Option<&std::path::Path>,
    _verbose: bool,
) {
    eprintln!("✗ cargo codesign linux requires Linux");
    std::process::exit(3);
}

fn cmd_verify(
    artifact: &std::path::Path,
    method: &str,
    signature: Option<&std::path::Path>,
    public_key: Option<&std::path::Path>,
    verbose: bool,
) {
    if !artifact.exists() {
        eprintln!("✗ Artifact not found: {}", artifact.display());
        std::process::exit(1);
    }

    let sig_path_buf;
    let sig_path = if let Some(p) = signature {
        p
    } else {
        sig_path_buf = cargo_codesign::verify::default_signature_path(artifact, method);
        &sig_path_buf
    };

    match method {
        "macos" => cmd_verify_macos(artifact, verbose),
        "windows" => cmd_verify_windows(artifact, verbose),
        "update" => cmd_verify_update(artifact, sig_path, public_key),
        "cosign" | "minisign" | "gpg" => {
            cmd_verify_linux(artifact, method, sig_path, public_key, verbose);
        }
        other => {
            eprintln!("✗ Unknown method: {other}");
            eprintln!("  Supported: macos, windows, update, cosign, minisign, gpg");
            std::process::exit(2);
        }
    }
}

#[cfg(target_os = "macos")]
fn cmd_verify_macos(artifact: &std::path::Path, verbose: bool) {
    eprintln!("Verifying macOS code signature...");
    cargo_codesign::platform::macos::verify_codesign(artifact, verbose).unwrap_or_else(|e| {
        eprintln!("✗ Codesign verification failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ codesign --verify passed");

    eprintln!("Checking Gatekeeper assessment...");
    cargo_codesign::platform::macos::verify_gatekeeper(artifact, verbose).unwrap_or_else(|e| {
        eprintln!("✗ Gatekeeper assessment failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ spctl --assess passed");
    eprintln!("✓ Verified: {}", artifact.display());
}

#[cfg(not(target_os = "macos"))]
fn cmd_verify_macos(_artifact: &std::path::Path, _verbose: bool) {
    eprintln!("✗ macOS verification requires macOS");
    std::process::exit(3);
}

#[cfg(target_os = "windows")]
fn cmd_verify_windows(artifact: &std::path::Path, verbose: bool) {
    eprintln!("Verifying Windows signature...");
    cargo_codesign::platform::windows::verify_exe(artifact, verbose).unwrap_or_else(|e| {
        eprintln!("✗ Signature verification failed: {e}");
        std::process::exit(1);
    });
    eprintln!("✓ Verified: {}", artifact.display());
}

#[cfg(not(target_os = "windows"))]
fn cmd_verify_windows(_artifact: &std::path::Path, _verbose: bool) {
    eprintln!("✗ Windows verification requires Windows");
    std::process::exit(3);
}

fn cmd_verify_update(
    artifact: &std::path::Path,
    sig_path: &std::path::Path,
    public_key: Option<&std::path::Path>,
) {
    let pub_key_path = public_key.unwrap_or_else(|| {
        eprintln!("✗ --public-key is required for update verification");
        std::process::exit(2);
    });
    let pub_key_b64 = std::fs::read_to_string(pub_key_path).unwrap_or_else(|e| {
        eprintln!(
            "✗ Failed to read public key {}: {e}",
            pub_key_path.display()
        );
        std::process::exit(1);
    });
    eprintln!("Verifying ed25519 signature...");
    cargo_codesign::update::verify_file(artifact, sig_path, pub_key_b64.trim()).unwrap_or_else(
        |e| {
            eprintln!("✗ Verification failed: {e}");
            std::process::exit(1);
        },
    );
    eprintln!("✓ Verified: {}", artifact.display());
}

fn cmd_verify_linux(
    artifact: &std::path::Path,
    method: &str,
    sig_path: &std::path::Path,
    public_key: Option<&std::path::Path>,
    verbose: bool,
) {
    if !sig_path.exists() {
        eprintln!("✗ Signature file not found: {}", sig_path.display());
        eprintln!("  Use --signature to specify the path");
        std::process::exit(1);
    }

    match method {
        "cosign" => {
            eprintln!("Verifying cosign bundle...");
            cargo_codesign::platform::linux::verify_cosign(artifact, sig_path, verbose)
                .unwrap_or_else(|e| {
                    eprintln!("✗ Verification failed: {e}");
                    std::process::exit(1);
                });
        }
        "minisign" => {
            let pub_key_path = public_key.unwrap_or_else(|| {
                eprintln!("✗ --public-key is required for minisign verification");
                std::process::exit(2);
            });
            let pub_key = std::fs::read_to_string(pub_key_path).unwrap_or_else(|e| {
                eprintln!("✗ Failed to read public key: {e}");
                std::process::exit(1);
            });
            eprintln!("Verifying minisign signature...");
            cargo_codesign::platform::linux::verify_minisign(
                artifact,
                sig_path,
                pub_key.trim(),
                verbose,
            )
            .unwrap_or_else(|e| {
                eprintln!("✗ Verification failed: {e}");
                std::process::exit(1);
            });
        }
        "gpg" => {
            eprintln!("Verifying GPG signature...");
            cargo_codesign::platform::linux::verify_gpg(artifact, sig_path, verbose)
                .unwrap_or_else(|e| {
                    eprintln!("✗ Verification failed: {e}");
                    std::process::exit(1);
                });
        }
        _ => unreachable!(),
    }
    eprintln!("✓ Verified: {}", artifact.display());
}

fn cmd_status(config_path: Option<&std::path::Path>) {
    let _ = dotenvy::dotenv();

    let (config, resolved_path, warnings) = if let Some(path) = config_path {
        cargo_codesign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_codesign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    };

    for w in &warnings {
        eprintln!("{w}");
    }
    eprintln!("Using config: {}", resolved_path.display());
    eprintln!();

    let report = cargo_codesign::status::check_status(&config);

    for check in &report.checks {
        if check.passed {
            eprintln!("  ✓ {}: {}", check.name, check.detail);
        } else {
            eprintln!("  ✗ {}: {}", check.name, check.detail);
        }
    }

    eprintln!();
    if report.all_passed() {
        eprintln!("All checks passed.");
    } else {
        let failed = report.checks.iter().filter(|c| !c.passed).count();
        eprintln!("{failed} check(s) failed.");
        std::process::exit(4);
    }
}

fn cmd_ci(config_path: Option<&std::path::Path>, output: &std::path::Path) {
    let _ = dotenvy::dotenv();

    let (config, resolved_path, warnings) = if let Some(path) = config_path {
        cargo_codesign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_codesign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    };

    for w in &warnings {
        eprintln!("{w}");
    }
    eprintln!("Using config: {}", resolved_path.display());

    let yaml = cargo_codesign::ci::generate_workflow(&config);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("✗ Failed to create directory {}: {e}", parent.display());
            std::process::exit(1);
        });
    }

    std::fs::write(output, &yaml).unwrap_or_else(|e| {
        eprintln!("✗ Failed to write {}: {e}", output.display());
        std::process::exit(1);
    });

    eprintln!("✓ Generated: {}", output.display());
}

fn cmd_keygen(output_private: &std::path::Path, output_public: &std::path::Path) {
    let (private_b64, public_b64) =
        cargo_codesign::keygen::generate_keypair().expect("failed to generate keypair");
    std::fs::write(output_private, format!("{private_b64}\n"))
        .expect("failed to write private key");
    std::fs::write(output_public, format!("{public_b64}\n")).expect("failed to write public key");
    eprintln!("✓ Keypair generated");
    eprintln!("  Private key: {}", output_private.display());
    eprintln!("  Public key:  {}", output_public.display());
    eprintln!();
    match cargo_codesign::keygen::update_gitignore(output_private) {
        Ok(true) => eprintln!("✓ Added private key to .gitignore"),
        Ok(false) => eprintln!("  Private key already listed in .gitignore"),
        Err(e) => eprintln!("  Warning: could not update .gitignore: {e}"),
    }
    eprintln!();
    eprintln!("⚠️  SECURITY WARNING");
    eprintln!("   NEVER commit the private key to git or share it with anyone!");
    eprintln!("   Treat it like a password — store it only in your CI secrets.");
    eprintln!();
    eprintln!("Store the private key as UPDATE_SIGNING_KEY in your CI secrets.");
    eprintln!("Embed the public key in your binary at compile time:");
    eprintln!("  const UPDATE_PUBLIC_KEY: &str = include_str!(\"../update-signing.pub\");");
}

fn cmd_update(
    archive: &std::path::Path,
    output: Option<&std::path::Path>,
    key_env: &str,
    public_key: Option<&std::path::Path>,
) {
    let _ = dotenvy::dotenv();
    let private_key_b64 = std::env::var(key_env).unwrap_or_else(|_| {
        eprintln!("✗ Environment variable {key_env} not set");
        eprintln!("  Set it in .env or export it: export {key_env}=<base64>");
        std::process::exit(1);
    });
    let sig_path = if let Some(p) = output {
        p.to_path_buf()
    } else {
        let mut p = archive.as_os_str().to_owned();
        p.push(".sig");
        std::path::PathBuf::from(p)
    };
    cargo_codesign::update::sign_file(archive, &sig_path, &private_key_b64).unwrap_or_else(|e| {
        eprintln!("✗ Signing failed: {e}");
        std::process::exit(1);
    });
    eprintln!("✓ Signed: {}", sig_path.display());

    if let Some(pub_key_path) = public_key {
        let pub_key_b64 = std::fs::read_to_string(pub_key_path).unwrap_or_else(|e| {
            eprintln!(
                "✗ Failed to read public key {}: {e}",
                pub_key_path.display()
            );
            std::process::exit(1);
        });
        cargo_codesign::update::verify_file(archive, &sig_path, pub_key_b64.trim()).unwrap_or_else(
            |e| {
                eprintln!("✗ Verification failed: {e}");
                std::process::exit(1);
            },
        );
        eprintln!("✓ Verified against {}", pub_key_path.display());
    }
}

fn cmd_init() {
    use cargo_codesign::config::{LinuxMethod, MacosAuth};
    use cargo_codesign::init::{
        check_credentials, generate_sign_toml, print_credential_report, InitSelections,
    };
    use dialoguer::{Confirm, MultiSelect, Select};

    let sign_toml = std::path::Path::new("sign.toml");
    if sign_toml.exists() {
        let overwrite = Confirm::new()
            .with_prompt("sign.toml already exists. Overwrite?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !overwrite {
            eprintln!("Aborted.");
            return;
        }
    }

    let platforms = &["macOS", "Windows", "Linux", "Update signing (ed25519)"];
    let selected = MultiSelect::new()
        .with_prompt("Which platforms will you sign for? (Space to select, Enter to confirm)")
        .items(platforms)
        .interact()
        .unwrap_or_default();

    if selected.is_empty() {
        eprintln!("No platforms selected. Aborted.");
        return;
    }

    let has_macos = selected.contains(&0);
    let has_windows = selected.contains(&1);
    let has_linux = selected.contains(&2);
    let has_update = selected.contains(&3);

    let macos_auth = if has_macos {
        let auth_modes = &["apple-id (local/indie dev)", "api-key (CI/team)"];
        let choice = Select::new()
            .with_prompt("macOS auth mode")
            .items(auth_modes)
            .default(0)
            .interact()
            .unwrap_or(0);
        Some(if choice == 0 {
            MacosAuth::AppleId
        } else {
            MacosAuth::ApiKey
        })
    } else {
        None
    };

    let linux_method = if has_linux {
        let methods = &[
            "cosign (keyless OIDC, recommended for CI)",
            "minisign (self-managed keys)",
            "gpg",
        ];
        let choice = Select::new()
            .with_prompt("Linux signing method")
            .items(methods)
            .default(0)
            .interact()
            .unwrap_or(0);
        Some(match choice {
            0 => LinuxMethod::Cosign,
            1 => LinuxMethod::Minisign,
            _ => LinuxMethod::Gpg,
        })
    } else {
        None
    };

    let selections = InitSelections {
        macos: has_macos,
        macos_auth,
        windows: has_windows,
        linux: has_linux,
        linux_method,
        update: has_update,
    };

    let toml_content = generate_sign_toml(&selections);
    std::fs::write(sign_toml, &toml_content).unwrap_or_else(|e| {
        eprintln!("✗ Failed to write sign.toml: {e}");
        std::process::exit(1);
    });
    eprintln!("✓ Created sign.toml");
    eprintln!();

    let _ = dotenvy::dotenv();
    let checks = check_credentials(&selections);
    let missing = checks.iter().filter(|c| !c.is_set).count();

    if missing > 0 {
        eprintln!("Credential status ({missing} missing):");
        print_credential_report(&checks);
        eprintln!();
        eprintln!("Set missing credentials in .env or CI secrets, then run:");
    } else {
        eprintln!("All credentials are set! Run:");
    }
    eprintln!("  cargo codesign status");
}
