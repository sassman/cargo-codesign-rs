# Setting Up Linux Credentials

The credentials you need depend on the signing method configured in `sign.toml`.

## cosign (keyless OIDC)

Recommended for GitHub Actions — uses [Sigstore](https://sigstore.dev) keyless signing via OIDC identity. **No long-lived keys required.**

In CI, cosign automatically uses the GitHub Actions OIDC token. Locally, it opens a browser for authentication.

If you prefer key-based signing:

```bash
cosign generate-key-pair
# Writes cosign.key (private) and cosign.pub (public)
```

Set `COSIGN_PRIVATE_KEY` to the contents of `cosign.key` (or store in CI secrets).

**Install cosign:** <https://docs.sigstore.dev/cosign/system_config/installation/>

## minisign (self-managed keys)

Generate a keypair:

```bash
minisign -G -s minisign.key -p minisign.pub
```

Set `MINISIGN_PRIVATE_KEY` to the contents of `minisign.key`.

**Install minisign:** `cargo install minisign` or <https://jedisct1.github.io/minisign/>

## gpg

Use your existing GPG key:

```bash
gpg --list-secret-keys --keyid-format LONG
# Export for CI:
gpg --armor --export-secret-keys YOUR_KEY_ID | base64
```

Set `GPG_PRIVATE_KEY` to the base64-encoded armor output.

## Verify

After setting credentials, run:

```bash
cargo codesign status
```
