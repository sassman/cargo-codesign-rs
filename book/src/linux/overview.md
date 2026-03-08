# Linux Signing

Sign Linux release archives using one of three methods: **cosign** (keyless OIDC), **minisign** (self-managed keys), or **gpg** (detached signatures).

## Usage

```bash
cargo codesign linux --archive target/release/myapp.tar.gz
```

The signing method is determined by `[linux] method` in `sign.toml`. Override it at the command line:

```bash
cargo codesign linux --archive release.tar.gz --method cosign
cargo codesign linux --archive release.tar.gz --method minisign
cargo codesign linux --archive release.tar.gz --method gpg
```

Specify a custom output path for the signature file:

```bash
cargo codesign linux --archive release.tar.gz --output release.tar.gz.cosign-bundle
```

## Methods

### cosign (keyless OIDC)

Recommended for GitHub Actions. Uses [Sigstore](https://sigstore.dev) keyless signing via OIDC — no private key management required.

Produces a `.bundle` file alongside the archive.

### minisign

Self-managed key signing via [minisign](https://jedisct1.github.io/minisign/). The private key is read from the environment variable configured in `[linux.env] key`.

Produces a `.minisig` file alongside the archive.

### gpg

Standard GPG detached signatures. Uses the default GPG key on the system.

Produces a `.sig` file alongside the archive.

## Configuration

```toml
[linux]
method = "cosign"

[linux.env]
key = "COSIGN_PRIVATE_KEY"
```

See the [sign.toml Reference](../reference/sign-toml.md) for full details, and [Setting Up Credentials](./credentials.md) for how to obtain signing credentials.
