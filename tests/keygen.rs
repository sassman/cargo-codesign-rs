use base64::{engine::general_purpose::STANDARD, Engine};
use cargo_codesign::keygen::generate_keypair;
use ed25519_dalek::{SigningKey, VerifyingKey};

#[test]
fn generate_keypair_produces_valid_keys() {
    let (private_b64, public_b64) = generate_keypair().unwrap();

    let private_bytes = STANDARD.decode(&private_b64).unwrap();
    let public_bytes = STANDARD.decode(&public_b64).unwrap();

    assert_eq!(private_bytes.len(), 32);
    assert_eq!(public_bytes.len(), 32);

    let signing_key = SigningKey::from_bytes(&private_bytes.try_into().unwrap());
    let expected_public = signing_key.verifying_key();
    let actual_public = VerifyingKey::from_bytes(&public_bytes.try_into().unwrap()).unwrap();
    assert_eq!(expected_public, actual_public);
}

#[test]
fn generate_keypair_produces_different_keys_each_time() {
    let (key1, _) = generate_keypair().unwrap();
    let (key2, _) = generate_keypair().unwrap();
    assert_ne!(key1, key2);
}
