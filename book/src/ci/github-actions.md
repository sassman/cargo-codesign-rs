# GitHub Actions Walkthrough

This guide shows how to set up macOS code signing in GitHub Actions using cargo-codesign.

## Prerequisites

Configure these GitHub Secrets in your repository:

| Secret | Description |
|--------|-------------|
| `APPLE_CERTIFICATE_BASE64` | Base64-encoded `.p12` certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `APPLE_ID` | Your Apple ID email (for `apple-id` auth) |
| `APPLE_TEAM_ID` | Your 10-character team ID |
| `APPLE_APP_PASSWORD` | App-specific password |

## Workflow

```yaml
name: Release macOS

on:
  push:
    tags: ["v*"]

jobs:
  build-and-sign:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: cargo build --release -p my-app

      - name: Bundle .app
        run: ./scripts/bundle-macos.sh

      - name: Install cargo-codesign
        run: cargo install cargo-codesign

      - name: Import certificate
        run: cargo codesign macos --ci-import-cert
        env:
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE_BASE64 }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}

      - name: Sign, package, and notarize
        run: cargo codesign macos --app "target/release/bundle/MyApp.app" --verbose
        env:
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}

      - name: Cleanup certificate
        if: always()
        run: cargo codesign macos --ci-cleanup-cert

      - name: Upload DMG
        uses: softprops/action-gh-release@v2
        with:
          files: target/release/bundle/MyApp.dmg
```

## Key points

- **`--ci-import-cert`** reads the certificate env var names from `sign.toml`, base64-decodes the certificate, creates an ephemeral keychain, and imports it. No shell needed.
- **`--ci-cleanup-cert`** deletes the ephemeral keychain created by `--ci-import-cert`. Runs `if: always()` so cleanup happens even if signing fails. Safe to call when no keychain exists (logs a warning, exits 0).
- **`cargo codesign macos --app`** handles the full sign → DMG → notarize → staple chain.
- The env var names (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`) come from your `sign.toml`. The GitHub secret names (e.g. `APPLE_CERTIFICATE_BASE64`) can be whatever you prefer.

## Composing with cargo-dist

If you use [cargo-dist](https://opensource.axo.dev/cargo-dist/) for releases, add the signing steps after the build job:

```yaml
sign:
  needs: [build]
  runs-on: macos-latest
  steps:
    - name: Import certificate
      run: cargo codesign macos --ci-import-cert
      env:
        APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE_BASE64 }}
        APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
    - name: Sign macOS artifacts
      run: cargo codesign macos --app "path/to/MyApp.app" --verbose
    - name: Cleanup
      if: always()
      run: cargo codesign macos --ci-cleanup-cert
```
