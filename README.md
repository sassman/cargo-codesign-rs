<p align="center">
  <img src="assets/banner.svg" alt="cargo sign" width="820"/>
</p>

<p align="center">
  <a href="https://crates.io/crates/cargo-sign"><img src="https://img.shields.io/crates/v/cargo-sign.svg" alt="crates.io"/></a>
  <a href="https://github.com/steganogram/cargo-sign/blob/main/LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue" alt="license"/></a>
</p>

---

A cargo subcommand that handles code signing, notarization, and update signatures for Rust binaries across macOS, Windows, and Linux.

## Quick start

```sh
cargo install cargo-sign
cargo sign init          # generate sign.toml
cargo sign status        # check credentials and tools
cargo sign macos --app target/release/bundle/MyApp.app
```

## Documentation

Full documentation is available in the [cargo-sign book](https://steganogram.github.io/cargo-sign/) (coming soon).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
