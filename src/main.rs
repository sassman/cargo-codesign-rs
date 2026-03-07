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
    Macos,
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
        SignCommand::Status => eprintln!("cargo sign status: not yet implemented"),
        SignCommand::Macos => eprintln!("cargo sign macos: not yet implemented"),
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
