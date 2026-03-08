# macOS Signing Overview

Shipping a macOS app to users outside the App Store requires three steps:

1. **Code signing** — digitally sign your binary with a Developer ID certificate so macOS recognizes it as trusted
2. **Notarization** — submit the signed artifact to Apple's servers for automated malware scanning
3. **Stapling** — attach the notarization ticket to the artifact so it works offline

Skip any of these and users see the dreaded "Apple could not verify" dialog — or worse, Gatekeeper blocks the app entirely.

## What cargo-codesign handles

`cargo codesign macos` runs the full pipeline:

```
.app bundle ──► sign inner binaries ──► sign .app ──► create DMG
                                                          │
                                            sign DMG ◄────┘
                                                │
                                          notarize DMG
                                                │
                                           staple DMG
                                                │
                                         ✓ Ready to ship
```

## Before you start

You need:

1. **A Developer ID Application certificate** — see [Setting Up Credentials](./credentials.md)
2. **A built `.app` bundle** — cargo-codesign does not build or bundle. See below.
3. **Apple credentials for notarization** — either an API key or Apple ID + app-specific password

## Building a .app bundle (not a cargo-codesign concern)

Before you can sign, you need a `.app` bundle. Here's what that looks like:

```bash
# Minimal .app structure
mkdir -p "MyApp.app/Contents/MacOS"
mkdir -p "MyApp.app/Contents/Resources"

# Copy your binary
cp target/release/myapp "MyApp.app/Contents/MacOS/myapp"

# Copy metadata
cp Info.plist "MyApp.app/Contents/"
cp AppIcon.icns "MyApp.app/Contents/Resources/"
```

Tools like [cargo-bundle](https://github.com/nickelpack/cargo-bundle) and [cargo-packager](https://github.com/nickelpack/cargo-packager) automate this. Your project may also have a custom bundle script.

For universal binaries (Intel + Apple Silicon), create both architectures and combine them before bundling:

```bash
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

lipo -create \
  target/x86_64-apple-darwin/release/myapp \
  target/aarch64-apple-darwin/release/myapp \
  -output target/universal-apple-darwin/release/myapp
```

Once you have a `.app`, cargo-codesign takes over.

## Two modes

| Mode | Command | What it does |
|------|---------|-------------|
| **App mode** | `cargo codesign macos --app "MyApp.app"` | Full chain: sign app → create DMG → sign DMG → notarize → staple |
| **Bare binary mode** | `cargo codesign macos` | Discover binaries via `cargo metadata`, sign each, copy to `target/signed/` |

Most GUI apps use **app mode**. CLI tools distributed as standalone binaries use **bare binary mode**.
