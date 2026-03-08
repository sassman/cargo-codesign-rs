# Installation

## From source (recommended during early development)

```bash
cargo install --git https://github.com/sassman/cargo-codesign-rs
```

Or if cargo-codesign has been published:

```bash
cargo install cargo-codesign
```

## Verify installation

```bash
cargo codesign --version
```

## Prerequisites

cargo-codesign orchestrates platform-specific signing tools. You need the tools for your target platform:

| Platform | Required tools | How to get them |
|----------|---------------|-----------------|
| macOS | `codesign`, `xcrun`, `hdiutil` | Xcode Command Line Tools: `xcode-select --install` |
| Windows | `signtool.exe` | Windows SDK |
| Linux | `cosign` or `minisign` | See [Linux guide](../linux/overview.md) |

cargo-codesign itself is a single Rust binary with no runtime dependencies beyond these platform tools.
