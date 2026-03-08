# GitHub Actions Walkthrough

This guide shows how to set up macOS code signing in GitHub Actions using cargo-codesign.

## Prerequisites

Configure these GitHub Secrets in your repository:

| Secret | Description |
|--------|-------------|
| `APPLE_CERTIFICATE_BASE64` | Base64-encoded `.p12` certificate |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` |
| `KEYCHAIN_PASSWORD` | Any random string (for the ephemeral CI keychain) |
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
        env:
          APPLE_CERTIFICATE_BASE64: ${{ secrets.APPLE_CERTIFICATE_BASE64 }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          KEYCHAIN_PASSWORD: ${{ secrets.KEYCHAIN_PASSWORD }}
        run: |
          if [ -z "${APPLE_CERTIFICATE_BASE64:-}" ]; then
            echo "::warning::Code signing secrets not configured"
            exit 0
          fi

          echo "$APPLE_CERTIFICATE_BASE64" | base64 --decode > certificate.p12

          security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
          security default-keychain -s build.keychain
          security unlock-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
          security set-keychain-settings -t 3600 -u build.keychain

          security import certificate.p12 \
            -k build.keychain \
            -P "$APPLE_CERTIFICATE_PASSWORD" \
            -T /usr/bin/codesign \
            -T /usr/bin/security

          security set-key-partition-list \
            -S apple-tool:,apple: \
            -s -k "$KEYCHAIN_PASSWORD" build.keychain

          rm certificate.p12

      - name: Sign, package, and notarize
        env:
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          APPLE_APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
        run: |
          cargo codesign macos --app "target/release/bundle/MyApp.app" --verbose

      - name: Clean up keychain
        if: always()
        run: security delete-keychain build.keychain 2>/dev/null || true

      - name: Upload DMG
        uses: softprops/action-gh-release@v2
        with:
          files: target/release/bundle/MyApp.dmg
```

## Key points

- **Certificate import** happens before `cargo codesign` because CI runners don't have your Keychain. This step creates an ephemeral keychain, imports the certificate, and allows `codesign` to access it.
- **`cargo codesign macos --app`** replaces what would otherwise be 3-4 separate shell scripts (codesign, create-dmg, notarize, staple).
- **Keychain cleanup** runs `if: always()` to clean up even if signing fails.
- **Graceful degradation**: if secrets aren't configured, the workflow skips signing and produces an unsigned build.

## Composing with cargo-dist

If you use [cargo-dist](https://opensource.axo.dev/cargo-dist/) for releases, add the signing step after the build job:

```yaml
sign:
  needs: [build]
  runs-on: macos-latest
  steps:
    # ... certificate import ...
    - name: Sign macOS artifacts
      run: cargo codesign macos --app "path/to/MyApp.app" --verbose
```
