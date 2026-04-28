# Changelog

All notable changes to this project will be documented in this file.

## [0.2.4] - 2026-04-28

### Bug Fixes

- Prepend ephemeral keychain to user search list (closes #34) ([#35](https://github.com/sassman/cargo-codesign-rs/pull/35))



## [0.2.3] - 2026-04-28

### Bug Fixes

- Pass --keychain explicitly so CI sign step finds imported identity ([#33](https://github.com/sassman/cargo-codesign-rs/pull/33))

### Miscellaneous

- Bump actions/upload-pages-artifact from 4 to 5 ([#28](https://github.com/sassman/cargo-codesign-rs/pull/28))
- Bump mozilla-actions/sccache-action from 0.0.9 to 0.0.10 ([#31](https://github.com/sassman/cargo-codesign-rs/pull/31))

### Security

- Bump plist from 1.8.0 to 1.9.0 ([#32](https://github.com/sassman/cargo-codesign-rs/pull/32))

### Deps

- Bump clap from 4.6.0 to 4.6.1 ([#30](https://github.com/sassman/cargo-codesign-rs/pull/30))



## [0.2.2] - 2026-04-10

### Bug Fixes

- Prevent keychain password disclosure during CI import ([#26](https://github.com/sassman/cargo-codesign-rs/pull/26))



## [0.2.1] - 2026-04-06

### Features

- Auto-add private key to `.gitignore` on keygen ([#22](https://github.com/sassman/cargo-codesign-rs/pull/22))

### Miscellaneous

- Bump actions/deploy-pages from 4 to 5 ([#20](https://github.com/sassman/cargo-codesign-rs/pull/20))

### Security

- Bump toml from 1.0.7+spec-1.1.0 to 1.1.2+spec-1.1.0 ([#24](https://github.com/sassman/cargo-codesign-rs/pull/24))



## [0.2.0] - 2026-03-13

### Features

- Native .DS_Store writer for styled DMG installers ([#14](https://github.com/sassman/cargo-codesign-rs/pull/14))

### Bug Fixes

- Fix DMG to include `/Applications` symlink ([#11](https://github.com/sassman/cargo-codesign-rs/pull/11))

### Security

- Bump clap from 4.5.60 to 4.6.0 ([#13](https://github.com/sassman/cargo-codesign-rs/pull/13))



## [0.1.2] - 2026-03-09

### Bug Fixes

- Release pipeline fails when unix/windows archive outputs are empty



## [0.1.1] - 2026-03-09

### Features

- Add --ci-import-cert and --ci-cleanup-cert to macos subcommand ([#6](https://github.com/sassman/cargo-codesign-rs/pull/6))

### Miscellaneous

- Add MIT and Apache-2.0 license files
- Add release binary assets pipeline
- Bump libc 0.2.182 → 0.2.183
- Add dependabot for nightly cargo and actions updates
- Bump actions/checkout from 4 to 6 ([#3](https://github.com/sassman/cargo-codesign-rs/pull/3))
- Bump actions/upload-pages-artifact from 3 to 4 ([#4](https://github.com/sassman/cargo-codesign-rs/pull/4))

### Deps

- Bump dialoguer, toml; replace rand with rand_core



## [0.1.0] - 2026-03-08

### Features

- Initial project scaffold with clap CLI skeleton
- Sign.toml config data model with serde parsing
- Config file resolution with sign.toml precedence over .cargo/sign.toml
- Ed25519 keypair generation for update signing
- Ed25519 update signing and verification
- Wire cargo sign keygen and update CLI subcommands
- Subprocess execution helper with verbose mode
- MacOS signing pipeline (codesign, DMG, notarize, staple)
- Cargo metadata binary discovery with signed output paths
- Dist-manifest.json parsing for cargo-dist integration
- Cargo sign status with env var and tool checks
- Wire cargo sign macos CLI with app, dmg, and bare binary modes
- Add README with wax-seal banner and project introduction
- TOML generation for cargo codesign init
- Credential walkthrough with env var checks and help links
- Wire cargo codesign init with interactive prompts and credential walkthrough
- Add GitHub Actions workflow templates
- GitHub Actions workflow generation from sign.toml
- Wire cargo codesign workflow CLI
- Windows signing pipeline via Azure Trusted Signing
- Wire cargo codesign windows CLI
- Linux signing pipeline (cosign, minisign, gpg)
- Wire cargo codesign linux CLI
- Add verification functions for all platforms
- Wire cargo codesign verify CLI with multi-method dispatch

### Bug Fixes

- Box toml::de::Error to satisfy result_large_err lint
- Make path assertions platform-agnostic for Windows CI
- Fix fmt
- Resolve clippy lints on Linux CI
- Use correct spctl flags for DMG gatekeeper assessment

### Refactor

- Deduplicate help links in credential report

### Documentation

- Initialize mdbook scaffold with chapter structure
- Write introduction chapter
- Write getting started chapters (installation, init stub, status)
- Write macOS overview and credentials chapters
- Write macOS app mode and bare binary mode chapters
- Write macOS auth modes and troubleshooting chapters
- Write update signing chapters (keygen, signing, integration)
- Write CI integration chapters (GitHub Actions, secrets)
- Write reference chapters (CLI, sign.toml, env vars, exit codes)
- Write stub chapters for Windows and Linux signing
- Update init chapter with wizard walkthrough
- Add Windows and Linux credential setup chapters
- Update workflow generation chapter
- Update Windows and Linux overview chapters
- Add verify chapter and update CLI reference for all commands
- Remove coming soon note on the mdbook

### Styling

- Improve banner readability with sepia background and higher contrast

### Testing

- Add update verification roundtrip tests

### Miscellaneous

- Add GitHub Actions via shared reusable workflow
- Add release-plz for changelog PRs and tag-triggered crates.io publish
- Update repository URLs to sassman/cargo-codesign-rs

### Deps

- Add dialoguer for interactive init prompts

### Rename

- Cargo-sign → cargo-codesign
- Workflow → ci subcommand


