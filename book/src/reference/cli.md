# CLI Reference

## Global options

```
cargo codesign <COMMAND> [OPTIONS]

Options:
  --config <PATH>    Path to sign.toml [default: auto-discover]
  --verbose          Print subprocess commands and full output
  --dry-run          Validate and print what would be done, without executing
  --json             Machine-readable output
  -h, --help         Print help
  -V, --version      Print version
```

## Commands

### `cargo codesign status`

Validate credentials, certificates, and tool availability.

```
cargo codesign status
```

Checks all env vars and tools configured in `sign.toml`. Only checks credentials relevant to the configured auth mode. See [Checking Your Setup](../getting-started/status.md).

### `cargo codesign macos`

Sign, notarize, and staple macOS artifacts.

```
cargo codesign macos [OPTIONS]

Options:
  --app <PATH>           Path to .app bundle (full sign+DMG+notarize+staple chain)
  --dmg <PATH>           Path to existing DMG (codesign + notarize + staple)
  --entitlements <PATH>  Entitlements plist (overrides sign.toml)
  --identity <STRING>    Signing identity substring [default from config or "Developer ID Application"]
  --skip-notarize        Sign only, skip notarization
  --skip-staple          Skip stapling
  --ci-import-cert       CI: import base64 certificate into ephemeral keychain
  --ci-cleanup-cert      CI: delete ephemeral keychain from a previous import
```

**Modes:**
- `--app <PATH>`: Sign app → create DMG → sign DMG → notarize → staple
- `--dmg <PATH>`: Sign existing DMG → notarize → staple
- Neither: Discover binaries via `cargo metadata`, sign each, copy to `target/signed/`

### `cargo codesign windows`

Sign Windows executables via Azure Trusted Signing.

```
cargo codesign windows [OPTIONS]

Options:
  --install-tools    Download Azure Trusted Signing tools via NuGet
```

See [Windows Signing Guide](../windows/overview.md).

### `cargo codesign linux`

Sign a Linux artifact with cosign, minisign, or gpg.

```
cargo codesign linux [OPTIONS]

Options:
  --archive <PATH>   Archive to sign (required)
  --method <METHOD>  Override method from config: cosign, minisign, gpg
  --output <PATH>    Signature output path
```

See [Linux Signing Guide](../linux/overview.md).

### `cargo codesign keygen`

Generate an Ed25519 keypair for update signing.

```
cargo codesign keygen [OPTIONS]

Options:
  --output-private <PATH>  [default: ./update-signing.key]
  --output-public <PATH>   [default: ./update-signing.pub]
```

### `cargo codesign update`

Sign a release archive for in-app update verification (Ed25519).

```
cargo codesign update [OPTIONS]

Options:
  --archive <PATH>      Archive to sign (required)
  --output <PATH>       Signature output [default: <archive>.sig]
  --key-env <STRING>    Env var name for the signing key [default: UPDATE_SIGNING_KEY]
  --public-key <PATH>   Also verify with this public key after signing
```

### `cargo codesign verify`

Verify a signed artifact or signature file.

```
cargo codesign verify <ARTIFACT> --method <METHOD> [OPTIONS]

Options:
  --method <METHOD>       Verification method: macos, windows, update, cosign, minisign, gpg
  --signature <PATH>      Explicit signature/bundle file (auto-detected if omitted)
  --public-key <PATH>     Public key for update/minisign verification
```

See [Verifying Signatures](../reference/verify.md).

### `cargo codesign ci`

Generate CI workflow (GitHub Actions YAML) from sign.toml.

```
cargo codesign ci [OPTIONS]

Options:
  --output <PATH>    Output path [default: .github/workflows/release-sign.yml]
```

See [Workflow Generation](../ci/workflow-generation.md).

### `cargo codesign init`

Create sign.toml with guided interactive prompts.

```
cargo codesign init
```

See [Creating sign.toml](../getting-started/init.md).
