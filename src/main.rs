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
    Update,
    /// Verify a signed artifact or signature file
    Verify,
    /// Generate an ed25519 keypair for update signing
    Keygen,
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
        SignCommand::Update => eprintln!("cargo sign update: not yet implemented"),
        SignCommand::Verify => eprintln!("cargo sign verify: not yet implemented"),
        SignCommand::Keygen => eprintln!("cargo sign keygen: not yet implemented"),
        SignCommand::Workflow => eprintln!("cargo sign workflow: not yet implemented"),
        SignCommand::Init => eprintln!("cargo sign init: not yet implemented"),
    }
}
