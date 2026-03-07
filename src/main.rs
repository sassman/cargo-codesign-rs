use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Sign(SignArgs),
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
        #[arg(long)]
        app: Option<std::path::PathBuf>,
        /// Path to existing DMG (codesign + notarize + staple)
        #[arg(long)]
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
    },
    /// Sign a Windows executable via Azure Trusted Signing
    Windows,
    /// Sign a Linux artifact with cosign or minisign
    Linux,
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
    Verify,
    /// Generate an ed25519 keypair for update signing
    Keygen {
        /// Output path for private key
        #[arg(long, default_value = "./update-signing.key")]
        output_private: std::path::PathBuf,
        /// Output path for public key
        #[arg(long, default_value = "./update-signing.pub")]
        output_public: std::path::PathBuf,
    },
    /// Generate GitHub Actions YAML
    Workflow,
    /// Create sign.toml with guided prompts
    Init,
}

fn main() {
    let CargoCli::Sign(args) = CargoCli::parse();
    match args.command {
        SignCommand::Status => cmd_status(args.config.as_deref()),
        SignCommand::Macos {
            app,
            dmg,
            entitlements,
            identity,
            skip_notarize,
            skip_staple,
        } => cmd_macos(
            args.config.as_deref(),
            app.as_deref(),
            dmg.as_deref(),
            entitlements.as_deref(),
            identity.as_deref(),
            skip_notarize,
            skip_staple,
            args.verbose,
        ),
        SignCommand::Windows => eprintln!("cargo sign windows: not yet implemented"),
        SignCommand::Linux => eprintln!("cargo sign linux: not yet implemented"),
        SignCommand::Update {
            archive,
            output,
            key_env,
            public_key,
        } => cmd_update(&archive, output.as_deref(), &key_env, public_key.as_deref()),
        SignCommand::Verify => eprintln!("cargo sign verify: not yet implemented"),
        SignCommand::Keygen {
            output_private,
            output_public,
        } => cmd_keygen(&output_private, &output_public),
        SignCommand::Workflow => eprintln!("cargo sign workflow: not yet implemented"),
        SignCommand::Init => eprintln!("cargo sign init: not yet implemented"),
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
        cargo_sign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_sign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
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

    if let Some(dmg_path) = dmg {
        macos_dmg_mode(
            dmg_path,
            identity,
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
            macos_config,
            skip_notarize,
            skip_staple,
            verbose,
        );
    } else {
        macos_bare_binary_mode(identity, verbose);
    }
}

#[cfg(target_os = "macos")]
fn macos_dmg_mode(
    dmg_path: &std::path::Path,
    identity: &str,
    macos_config: &cargo_sign::config::MacosConfig,
    skip_notarize: bool,
    skip_staple: bool,
    verbose: bool,
) {
    use cargo_sign::platform::macos;

    eprintln!("[1/3] Codesigning DMG...");
    let opts = macos::CodesignOpts {
        identity,
        entitlements: None,
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
    macos_config: &cargo_sign::config::MacosConfig,
    skip_notarize: bool,
    skip_staple: bool,
    verbose: bool,
) {
    use cargo_sign::platform::macos;

    let app_name = app_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    eprintln!("[1/5] Codesigning .app bundle...");
    let opts = macos::CodesignOpts {
        identity,
        entitlements,
        verbose,
    };
    macos::codesign_app(app_path, &opts).unwrap_or_else(|e| {
        eprintln!("✗ App codesigning failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ App signed");

    let dmg_path = app_path.with_extension("dmg");
    eprintln!("[2/5] Creating DMG...");
    macos::create_dmg(app_path, &dmg_path, &app_name, verbose).unwrap_or_else(|e| {
        eprintln!("✗ DMG creation failed: {e}");
        std::process::exit(1);
    });
    eprintln!("  ✓ DMG created: {}", dmg_path.display());

    eprintln!("[3/5] Codesigning DMG...");
    let dmg_opts = macos::CodesignOpts {
        identity,
        entitlements: None,
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
fn macos_bare_binary_mode(identity: &str, verbose: bool) {
    use cargo_sign::platform::macos;

    eprintln!("Discovering binaries via cargo metadata...");
    let binaries = cargo_sign::discovery::discover_binaries().unwrap_or_else(|e| {
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
fn resolve_env(name: Option<&String>, field: &str) -> String {
    let env_name = name.unwrap_or_else(|| {
        eprintln!("✗ {field} not configured in sign.toml [macos.env]");
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
    macos_config: &cargo_sign::config::MacosConfig,
    verbose: bool,
) {
    use cargo_sign::platform::macos;

    match macos_config.auth {
        cargo_sign::config::MacosAuth::ApiKey => {
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
        cargo_sign::config::MacosAuth::AppleId => {
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

fn cmd_status(config_path: Option<&std::path::Path>) {
    let _ = dotenvy::dotenv();

    let (config, resolved_path, warnings) = if let Some(path) = config_path {
        cargo_sign::config::resolve::resolve_config_from_path(path).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    } else {
        cargo_sign::config::resolve::resolve_config(None).unwrap_or_else(|e| {
            eprintln!("✗ {e}");
            std::process::exit(2);
        })
    };

    for w in &warnings {
        eprintln!("{w}");
    }
    eprintln!("Using config: {}", resolved_path.display());
    eprintln!();

    let report = cargo_sign::status::check_status(&config);

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

fn cmd_keygen(output_private: &std::path::Path, output_public: &std::path::Path) {
    let (private_b64, public_b64) =
        cargo_sign::keygen::generate_keypair().expect("failed to generate keypair");
    std::fs::write(output_private, format!("{private_b64}\n"))
        .expect("failed to write private key");
    std::fs::write(output_public, format!("{public_b64}\n")).expect("failed to write public key");
    eprintln!("✓ Keypair generated");
    eprintln!("  Private key: {}", output_private.display());
    eprintln!("  Public key:  {}", output_public.display());
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
    cargo_sign::update::sign_file(archive, &sig_path, &private_key_b64).unwrap_or_else(|e| {
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
        cargo_sign::update::verify_file(archive, &sig_path, pub_key_b64.trim()).unwrap_or_else(
            |e| {
                eprintln!("✗ Verification failed: {e}");
                std::process::exit(1);
            },
        );
        eprintln!("✓ Verified against {}", pub_key_path.display());
    }
}
