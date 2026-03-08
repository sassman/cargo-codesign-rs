# Signing Release Archives

After building your release archive (`.tar.gz`, `.zip`, etc.), sign it with your Ed25519 private key:

```bash
cargo codesign update --archive target/release/myapp-v1.0.0.tar.gz
```

Output:

```
✓ Signed: target/release/myapp-v1.0.0.tar.gz.sig
```

## How it works

1. Reads the archive bytes
2. Signs with the Ed25519 private key from the `UPDATE_SIGNING_KEY` env var
3. Writes the base64-encoded signature to `<archive>.sig`

## Options

```bash
# Custom output path
cargo codesign update --archive release.tar.gz --output release.sig

# Use a different env var for the key
cargo codesign update --archive release.tar.gz --key-env MY_SIGNING_KEY

# Sign and verify in one step
cargo codesign update --archive release.tar.gz --public-key update-signing.pub
```

The `--public-key` flag tells cargo-codesign to verify the signature immediately after signing — a good sanity check.

## Providing the key

The private key is read from an environment variable (default: `UPDATE_SIGNING_KEY`). You can set it:

```bash
# Via .env file
echo "UPDATE_SIGNING_KEY=$(cat update-signing.key)" >> .env

# Or export directly
export UPDATE_SIGNING_KEY=$(cat update-signing.key)

# Then sign
cargo codesign update --archive release.tar.gz
```
