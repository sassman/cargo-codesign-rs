# Integrating with Your Updater

The update signing flow produces a `.sig` file alongside your release archive. Your in-app updater uses the public key (embedded at compile time) to verify the signature before applying the update.

## Embed the public key

```rust
const UPDATE_PUBLIC_KEY: &str = include_str!("../update-signing.pub");
```

## Verify in your updater

Using `ed25519-dalek` directly:

```rust
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

fn verify_update(archive_bytes: &[u8], signature_b64: &str) -> bool {
    let pub_bytes = STANDARD.decode(UPDATE_PUBLIC_KEY.trim()).unwrap();
    let pub_array: [u8; 32] = pub_bytes.try_into().unwrap();
    let verifying_key = VerifyingKey::from_bytes(&pub_array).unwrap();

    let sig_bytes = STANDARD.decode(signature_b64.trim()).unwrap();
    let sig_array: [u8; 64] = sig_bytes.try_into().unwrap();
    let signature = Signature::from_bytes(&sig_array);

    verifying_key.verify(archive_bytes, &signature).is_ok()
}
```

Or use cargo-codesign's library directly:

```rust
use cargo_codesign::update::verify_bytes;

let is_valid = verify_bytes(archive_bytes, signature_b64, UPDATE_PUBLIC_KEY.trim()).is_ok();
```

## Release workflow

1. Build release archive
2. `cargo codesign update --archive release.tar.gz --public-key update-signing.pub`
3. Upload both `release.tar.gz` and `release.tar.gz.sig` to your release
4. Your updater downloads both, verifies the signature, then applies
