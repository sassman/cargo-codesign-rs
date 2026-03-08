# Signing a GUI App (--app mode)

This is the most common mode for macOS GUI applications. It handles the full pipeline from `.app` bundle to signed, notarized DMG.

## Quick start

```bash
cargo codesign macos --app "target/release/bundle/MyApp.app"
```

This runs:

```
[1/5] Codesigning .app bundle...
  ✓ App signed
[2/5] Creating DMG...
  ✓ DMG created: target/release/bundle/MyApp.dmg
[3/5] Codesigning DMG...
  ✓ DMG signed
[4/5] Notarizing DMG...
  ✓ Notarized
[5/5] Stapling...
  ✓ Stapled
✓ Done: target/release/bundle/MyApp.dmg
```

## What happens at each step

### Step 1: Sign inner binaries and the .app bundle

cargo-codesign walks the `.app` bundle and signs:
1. Every binary in `Contents/MacOS/`
2. Every dylib and framework in `Contents/Frameworks/`
3. The `.app` bundle itself (with entitlements if configured)

Each is signed with `codesign --force --timestamp --options runtime`, which enables the hardened runtime required for notarization.

### Step 2: Create DMG

Creates a DMG from the `.app` using `hdiutil`. The DMG is placed next to the `.app`:

```
target/release/bundle/MyApp.app  →  target/release/bundle/MyApp.dmg
```

### Step 3: Sign the DMG

The DMG itself must also be codesigned — Apple's notarization service requires it.

### Step 4: Notarize

Submits the DMG to Apple's notarization service and waits for the result. This typically takes 1-5 minutes.

On failure, cargo-codesign prints the notarization log with specific issues (e.g., unsigned binary inside the bundle, forbidden entitlement).

### Step 5: Staple

Attaches the notarization ticket to the DMG so it works offline — users don't need to be online for Gatekeeper to verify.

## Skipping steps

For development builds where you don't need notarization:

```bash
# Sign only, skip notarization and stapling
cargo codesign macos --app "MyApp.app" --skip-notarize
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

# 3. Sign (cargo-codesign handles the rest)
cargo codesign macos --app "target/release/bundle/MyApp.app"
```

Or if you use a Makefile:

```makefile
dmg:
	cargo build --release --target aarch64-apple-darwin -p my-app
	./scripts/bundle-macos.sh
	set -a && source .env && set +a && \
		cargo codesign macos --app "target/release/bundle/MyApp.app"
```
