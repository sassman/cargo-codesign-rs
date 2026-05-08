# Signing a GUI App (--app mode)

This is the most common mode for macOS GUI applications. It handles the full pipeline from `.app` bundle to a signed, notarized, distributable artifact.

## Output formats

Use `--as` to choose how the final artifact is packaged:

| Flag | Output | Best for |
|------|--------|----------|
| `--as dmg` (default) | Signed + notarized DMG | Direct website downloads (drag-to-install) |
| `--as zip` | Zip containing stapled `.app` | Homebrew cask, Sparkle updaters |

In both cases the `.app` itself is signed, notarized, and stapled — the difference is only the outer container.

## Quick start

### Zip output (for Homebrew cask)

```bash
cargo codesign macos --app "MyApp.app" --as zip
```

```
[1/5] Codesigning .app bundle...
  ✓ App signed
[2/5] Zipping .app for notarization...
  ✓ Zip created: MyApp.zip
[3/5] Notarizing .app...
  ✓ Notarized
[4/5] Stapling .app...
  ✓ App stapled
[5/5] Creating distributable zip...
  ✓ Zip created
✓ Done: MyApp.zip
```

### DMG output (default)

```bash
cargo codesign macos --app "MyApp.app"
# or explicitly: cargo codesign macos --app "MyApp.app" --as dmg
```

```
[1/8] Codesigning .app bundle...
  ✓ App signed
[2/8] Zipping .app for notarization...
  ✓ Zip created: MyApp.zip
[3/8] Notarizing .app...
  ✓ Notarized
[4/8] Stapling .app...
  ✓ App stapled
[5/8] Creating DMG...
  ✓ DMG created: MyApp.dmg
[6/8] Codesigning DMG...
  ✓ DMG signed
[7/8] Notarizing DMG...
  ✓ Notarized
[8/8] Stapling DMG...
  ✓ DMG stapled
✓ Done: MyApp.dmg
```

Note: the DMG flow performs **two** notarization submissions (one for the `.app`, one for the DMG) so both carry stapled tickets.

## What happens at each step

### Sign .app bundle

cargo-codesign walks the `.app` bundle and signs:
1. Every binary in `Contents/MacOS/`
2. Every dylib and framework in `Contents/Frameworks/`
3. The `.app` bundle itself (with entitlements if configured)

Each is signed with `codesign --force --timestamp --options runtime`, enabling the hardened runtime required for notarization.

### Notarize .app

The `.app` is zipped and submitted to Apple's notarization service. This typically takes 1-5 minutes. On failure, cargo-codesign prints the notarization log with specific issues.

### Staple .app

Attaches the notarization ticket directly to the `.app` bundle. This ensures Gatekeeper passes even on machines that cannot reach Apple's servers (corporate firewalls, MDM-restricted Macs).

### Create DMG (dmg mode only)

Creates a DMG from the stapled `.app` using `hdiutil`. When a `[macos.dmg]` section is present in `sign.toml`, the DMG gets a styled installer window. See [DMG Styling](./dmg-styling.md).

### Sign + notarize + staple DMG (dmg mode only)

The DMG is codesigned, submitted for a second notarization, and stapled so direct-download users get offline Gatekeeper verification on the DMG itself.

## When to use which format

**Use `--as zip` when:**
- Distributing via Homebrew cask (cask extracts the `.app` from the zip)
- Using Sparkle or similar updater frameworks that expect a zip
- You want a single notarization (faster CI)

**Use `--as dmg` (default) when:**
- Users download from your website and expect drag-to-install
- You want a styled installer window with background image

You can produce both in CI by running the command twice with different `--as` flags.

## Skipping steps

For development builds where you don't need notarization:

```bash
cargo codesign macos --app "MyApp.app" --as zip --skip-notarize
```

## Overriding config from the command line

```bash
# Override signing identity
cargo codesign macos --app "MyApp.app" --identity "Developer ID Application: Other Team"

# Override entitlements
cargo codesign macos --app "MyApp.app" --entitlements custom-entitlements.plist

# See subprocess commands
cargo codesign macos --app "MyApp.app" --verbose
```

## Full example: build, bundle, sign

```bash
# 1. Build
cargo build --release --target aarch64-apple-darwin -p my-app

# 2. Bundle (your script or cargo-bundle)
./scripts/bundle-macos.sh

# 3. Sign — zip for Homebrew, dmg for website
cargo codesign macos --app "target/release/bundle/MyApp.app" --as zip
cargo codesign macos --app "target/release/bundle/MyApp.app" --as dmg
```
