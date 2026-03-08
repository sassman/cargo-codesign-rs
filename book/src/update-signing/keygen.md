# Generating Keypairs

Update signing uses Ed25519 (via [ed25519-dalek](https://crates.io/crates/ed25519-dalek)) to sign release archives. This is separate from OS-level code signing — it's for your in-app updater to verify that an update is authentic.

## Generate a keypair

```bash
cargo codesign keygen
```

Output:

```
✓ Keypair generated
  Private key: ./update-signing.key
  Public key:  ./update-signing.pub

Store the private key as UPDATE_SIGNING_KEY in your CI secrets.
Embed the public key in your binary at compile time:
  const UPDATE_PUBLIC_KEY: &str = include_str!("../update-signing.pub");
```

## Custom output paths

```bash
cargo codesign keygen \
  --output-private ./secrets/signing.key \
  --output-public ./keys/signing.pub
```

## Key format

Both keys are base64-encoded Ed25519 keys (32 bytes each, base64-encoded):

```
# update-signing.key (private — NEVER commit this)
dGhpcyBpcyBhIGZha2UgcHJpdmF0ZSBrZXk=

# update-signing.pub (public — safe to commit)
dGhpcyBpcyBhIGZha2UgcHVibGljIGtleQ==
```

## Security

- The private key must be kept secret. Store it as a CI secret (`UPDATE_SIGNING_KEY`), never in the repository.
- The public key is embedded in your binary at compile time. It's safe to commit.
- Keys are generated using `OsRng` (operating system's cryptographically secure random number generator).
