# Introduction

**cargo-codesign** is a cross-platform binary signing CLI for Rust projects. It replaces the collection of per-platform shell scripts that most teams maintain for code signing, notarization, and update integrity verification.

## What cargo-codesign does

- Signs macOS `.app` bundles and bare binaries with `codesign`
- Creates DMG installers and codesigns them
- Submits to Apple notarization and staples the ticket
- Signs release archives with Ed25519 for in-app update verification
- Generates Ed25519 keypairs for update signing
- Validates your credentials and tool setup before signing (`cargo codesign status`)
- Uses a single config file (`sign.toml`) that maps env var names to credentials

## What cargo-codesign does NOT do

- **Not a build tool.** It operates on already-built artifacts. It does not call `cargo build`.
- **Not a bundler.** It does not create `.app` bundles from raw binaries. That's [cargo-bundle](https://github.com/nickelpack/cargo-bundle), [cargo-packager](https://github.com/nickelpack/cargo-packager), or your own script. However, when given a `.app` via `--app`, it handles the full chain: sign, DMG, codesign DMG, notarize, staple.
- **Not a release orchestrator.** It does not bump versions, create tags, or publish to crates.io. That's [cargo-release](https://github.com/crate-ci/cargo-release) or [release-plz](https://github.com/MarcoIeni/release-plz).
- **Not a lipo/universal binary tool.** Universal binary creation (`lipo -create`) is a build step, not a signing step. However, this book documents how to do it as part of the end-to-end flow.

## The three signing layers

Binary signing has three distinct concerns:

| Layer | What | Purpose | Tool |
|-------|------|---------|------|
| **OS Trust** | macOS `codesign` + notarize, Windows `signtool` | OS runs the binary without warning/blocking | `cargo codesign macos`, `cargo codesign windows` |
| **Update Integrity** | Ed25519 signature over the release archive | In-app updater verifies authenticity | `cargo codesign update` |
| **Store Signing** | App Store, Microsoft Store submission | Store distribution | Out of scope for v1 |

A typical release pipeline needs Layer 1 + Layer 2. Layer 3 is separate and not covered by cargo-codesign.

## How to read this book

- **New to signing?** Start with [Installation](./getting-started/installation.md), then follow the [macOS Signing Guide](./macos/overview.md) end-to-end.
- **Setting up CI?** Jump to [GitHub Actions Walkthrough](./ci/github-actions.md).
- **Looking up a flag?** See the [CLI Reference](./reference/cli.md) or [sign.toml Reference](./reference/sign-toml.md).
