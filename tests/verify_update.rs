use cargo_codesign::update;
use cargo_codesign::verify::default_signature_path;
use std::path::Path;

#[test]
fn verify_update_roundtrip() {
    let dir = tempfile::TempDir::new().unwrap();
    let archive = dir.path().join("release.tar.gz");
    let sig = dir.path().join("release.tar.gz.sig");
    let pub_key_path = dir.path().join("update-signing.pub");

    // Create a fake archive
    std::fs::write(&archive, b"fake release content").unwrap();

    // Generate keypair
    let (private_b64, public_b64) = cargo_codesign::keygen::generate_keypair().unwrap();
    std::fs::write(&pub_key_path, &public_b64).unwrap();

    // Sign
    update::sign_file(&archive, &sig, &private_b64).unwrap();

    // Verify
    update::verify_file(&archive, &sig, &public_b64).unwrap();
}

#[test]
fn verify_update_wrong_key_fails() {
    let dir = tempfile::TempDir::new().unwrap();
    let archive = dir.path().join("release.tar.gz");
    let sig = dir.path().join("release.tar.gz.sig");

    std::fs::write(&archive, b"fake release content").unwrap();

    let (private_b64, _) = cargo_codesign::keygen::generate_keypair().unwrap();
    let (_, wrong_pub_b64) = cargo_codesign::keygen::generate_keypair().unwrap();

    update::sign_file(&archive, &sig, &private_b64).unwrap();
    let result = update::verify_file(&archive, &sig, &wrong_pub_b64);
    assert!(result.is_err());
}

#[test]
fn default_sig_path_for_update_is_dot_sig() {
    let path = default_signature_path(Path::new("app.tar.gz"), "update");
    assert_eq!(path.to_str().unwrap(), "app.tar.gz.sig");
}
