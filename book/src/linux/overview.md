# Linux Signing

> Linux signing via cosign and minisign is planned but not yet implemented.

When available, `cargo codesign linux` will support two methods:

## cosign (keyless OIDC)

Recommended for GitHub Actions. Uses [Sigstore](https://sigstore.dev) keyless signing via OIDC:

```bash
cargo codesign linux --archive release.tar.gz --method cosign
```

## minisign (self-managed keys)

Recommended for indie developers who prefer managing their own keys:

```bash
cargo codesign linux --archive release.tar.gz --method minisign
```

Configuration will use the `[linux]` and `[linux.env]` sections of `sign.toml`:

```toml
[linux]
method = "cosign"

[linux.env]
key = "COSIGN_PRIVATE_KEY"
```

See the [sign.toml Reference](../reference/sign-toml.md) for details.
