# Changelog

All notable changes to this project will be documented in this file.

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


