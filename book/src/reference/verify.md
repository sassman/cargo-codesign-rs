# Verifying Signatures

`cargo codesign verify` checks whether a signed artifact's signature is valid.

## Usage

```bash
cargo codesign verify <ARTIFACT> --method <METHOD> [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `--method <METHOD>` | Verification method (required): `macos`, `windows`, `update`, `cosign`, `minisign`, `gpg` |
| `--signature <PATH>` | Path to signature/bundle file. Auto-detected if omitted |
| `--public-key <PATH>` | Public key file. Required for `update` and `minisign` methods |

## Methods

### macOS

Runs `codesign --verify --deep --strict -vvv` and `spctl --assess` on the artifact. No signature file needed — macOS code signatures are embedded.

```bash
cargo codesign verify MyApp.app --method macos
cargo codesign verify MyApp.dmg --method macos
```

### Windows

Runs `signtool verify /pa /v` on the `.exe`. No signature file needed — Windows signatures are embedded.

```bash
cargo codesign verify myapp.exe --method windows
```

### update (ed25519)

Verifies a detached ed25519 signature created by `cargo codesign update`.

```bash
cargo codesign verify release.tar.gz --method update --public-key update-signing.pub
```

Default signature file: `<artifact>.sig`

### cosign

Verifies a Sigstore cosign bundle.

```bash
cargo codesign verify release.tar.gz --method cosign
```

Default signature file: `<artifact>.bundle`

### minisign

Verifies a minisign signature.

```bash
cargo codesign verify release.tar.gz --method minisign --public-key minisign.pub
```

Default signature file: `<artifact>.minisig`

### gpg

Verifies a GPG detached signature.

```bash
cargo codesign verify release.tar.gz --method gpg
```

Default signature file: `<artifact>.sig`

## Auto-detection

When `--signature` is omitted, the signature path is derived from the artifact path:

| Method | Default signature path |
|--------|----------------------|
| `update`, `gpg` | `<artifact>.sig` |
| `cosign` | `<artifact>.bundle` |
| `minisign` | `<artifact>.minisig` |
| `macos`, `windows` | Not applicable (embedded signatures) |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Verification passed |
| 1 | Verification failed or file not found |
| 2 | Bad arguments (unknown method, missing required flag) |
| 3 | Platform mismatch (e.g. `--method macos` on Linux) |
