<p align="center">
  <img src="assets/banner.svg" alt="cargo codesign" width="820"/>
</p>

<p align="center">
  <a href="https://crates.io/crates/cargo-codesign"><img src="https://img.shields.io/crates/v/cargo-codesign.svg" alt="crates.io"/></a>
  <a href="https://github.com/sassman/cargo-codesign-rs/blob/main/LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue" alt="license"/></a>
</p>

---

A cargo subcommand that handles code signing, notarization, and update signatures for Rust binaries across macOS, Windows, and Linux.

## Quick start

```sh
cargo install cargo-codesign
cargo codesign init          # generate sign.toml
cargo codesign status        # check credentials and tools
cargo codesign macos --app target/release/bundle/MyApp.app
```

## DMG Installer Styling (macOS)

Create polished drag-to-install DMG images with a background image and positioned icons — no AppleScript, no Finder, fully deterministic and CI-friendly.

Add a `[macos.dmg]` section to your `sign.toml`:

```toml
[macos.dmg]
background = "assets/dmg-background.png"
window-size = [660, 400]
icon-size = 128
app-position = [160, 200]
app-drop-link = [500, 200]
```

Then build as usual:

```sh
cargo codesign macos --app target/release/bundle/MyApp.app
```

The resulting DMG will show your background image with the app icon and an Applications folder symlink at the specified positions.

### Configuration Reference

| Field | Type | Description |
|-------|------|-------------|
| `background` | path | Background image (PNG recommended). Path relative to the directory where `cargo codesign` runs. |
| `window-size` | `[width, height]` | Finder window dimensions in pixels. |
| `icon-size` | integer | Icon size in the Finder window (px). |
| `app-position` | `[x, y]` | Position of the `.app` bundle icon. |
| `app-drop-link` | `[x, y]` | Position of the `Applications` folder symlink. |

All fields are required when the `[macos.dmg]` section is present. When the section is omitted entirely, a plain DMG without styling is created (previous default behavior).

### How It Works

Instead of the traditional mount-DMG → AppleScript → Finder → detach pipeline (slow, flaky in CI, requires a display), `cargo-codesign` writes a native `.DS_Store` file directly into the staging directory. The `.DS_Store` encodes icon positions, window properties, and the background image reference in macOS's buddy-allocator B-tree format. A single `hdiutil create -format UDZO` call then produces the final compressed DMG.

This approach is:
- **Fast** — no Finder launch, no AppleScript timeout
- **Deterministic** — same input always produces the same layout
- **CI-friendly** — works headless, no display server needed

## Documentation

Full documentation is available in the [cargo-codesign book](https://sassman.github.io/cargo-codesign-rs/).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
