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
```

**Modes:**
- `--app <PATH>`: Sign app → create DMG → sign DMG → notarize → staple
- `--dmg <PATH>`: Sign existing DMG → notarize → staple
- Neither: Discover binaries via `cargo metadata`, sign each, copy to `target/signed/`

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

### `cargo codesign windows`

> Not yet implemented.

### `cargo codesign linux`

> Not yet implemented.

### `cargo codesign verify`

> Not yet implemented.

### `cargo codesign workflow`

> Not yet implemented.

### `cargo codesign init`

> Not yet implemented.
