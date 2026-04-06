use base64::{engine::general_purpose::STANDARD, Engine};
use cargo_codesign::keygen::{generate_keypair, update_gitignore};
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

#[test]
fn update_gitignore_creates_file_and_adds_entry() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("update-signing.key");

    let added = update_gitignore(&key_path).unwrap();
    assert!(added, "expected entry to be added");

    let contents = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert!(contents.contains("update-signing.key"));
    assert!(contents.contains("# private key for codesign update signing:"));
}

#[test]
fn update_gitignore_does_not_duplicate_existing_entry() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("update-signing.key");
    let gitignore_path = dir.path().join(".gitignore");

    std::fs::write(&gitignore_path, "update-signing.key\n").unwrap();

    let added = update_gitignore(&key_path).unwrap();
    assert!(!added, "expected no change since entry already exists");

    let contents = std::fs::read_to_string(&gitignore_path).unwrap();
    assert_eq!(
        contents.matches("update-signing.key").count(),
        1,
        "entry should appear only once"
    );
}

#[test]
fn update_gitignore_appends_to_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("my-private.key");
    let gitignore_path = dir.path().join(".gitignore");

    std::fs::write(&gitignore_path, "/target\n").unwrap();

    let added = update_gitignore(&key_path).unwrap();
    assert!(added, "expected entry to be added");

    let contents = std::fs::read_to_string(&gitignore_path).unwrap();
    assert!(contents.contains("/target"));
    assert!(contents.contains("my-private.key"));
}
