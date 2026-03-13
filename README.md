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

- Styled DMG installers with background image and icon positions via `[macos.dmg]` in `sign.toml` — see [DMG Styling guide](https://sassman.github.io/cargo-codesign-rs/macos/dmg-styling.html)

## Documentation

Full documentation is available in the [cargo-codesign book](https://sassman.github.io/cargo-codesign-rs/).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
