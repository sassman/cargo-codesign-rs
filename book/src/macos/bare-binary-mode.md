# Signing a CLI Tool (bare binary mode)

If you're distributing a standalone CLI binary (not a `.app` bundle), use bare binary mode. cargo-codesign discovers your binaries via `cargo metadata` and signs each one.

## Quick start

```bash
# Build first
cargo build --release

# Sign all binary targets in the workspace
cargo codesign macos
```

This runs:

```
Discovering binaries via cargo metadata...
  Signing my-cli...
  ✓ my-cli → target/signed/release/my-cli
✓ Done
```

## How it works

1. Runs `cargo metadata --format-version 1 --no-deps` to find all binary targets
2. Looks for each binary in `target/release/`
3. Signs each with `codesign --force --timestamp --options runtime`
4. Copies signed binaries to `target/signed/release/`

The original binaries in `target/release/` are untouched.

## Limitations

- Bare binaries **cannot be stapled** — stapling only works on `.app` bundles and DMG files
- Notarization of bare binaries requires zipping them first, which cargo-codesign does not currently automate in this mode
- For full notarization, consider wrapping your CLI in a `.app` bundle or distributing as a signed DMG

## When to use this mode

- Distributing CLI tools via Homebrew taps (Homebrew handles its own verification)
- Internal tools where Gatekeeper isn't a concern
- As a pre-step before packaging into an installer
