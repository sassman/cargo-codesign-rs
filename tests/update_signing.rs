use base64::{engine::general_purpose::STANDARD, Engine};
use cargo_codesign::keygen::generate_keypair;
use cargo_codesign::update::{sign_bytes, verify_bytes};

#[test]
fn sign_and_verify_roundtrip() {
    let (private_b64, public_b64) = generate_keypair().unwrap();
    let data = b"hello world release archive";

    let signature_b64 = sign_bytes(data, &private_b64).unwrap();
    assert!(verify_bytes(data, &signature_b64, &public_b64).is_ok());
}

#[test]
fn verify_fails_with_wrong_public_key() {
    let (private_b64, _) = generate_keypair().unwrap();
    let (_, wrong_public_b64) = generate_keypair().unwrap();
    let data = b"hello world";

    let signature_b64 = sign_bytes(data, &private_b64).unwrap();
    assert!(verify_bytes(data, &signature_b64, &wrong_public_b64).is_err());
}

#[test]
fn verify_fails_with_tampered_data() {
    let (private_b64, public_b64) = generate_keypair().unwrap();
    let data = b"original data";
    let tampered = b"tampered data";

    let signature_b64 = sign_bytes(data, &private_b64).unwrap();
    assert!(verify_bytes(tampered, &signature_b64, &public_b64).is_err());
}

#[test]
fn sign_file_and_verify_file() {
    let (private_b64, public_b64) = generate_keypair().unwrap();
    let dir = tempfile::TempDir::new().unwrap();

    let archive_path = dir.path().join("release.tar.gz");
    let sig_path = dir.path().join("release.tar.gz.sig");

    std::fs::write(&archive_path, b"fake archive contents").unwrap();

    cargo_codesign::update::sign_file(&archive_path, &sig_path, &private_b64).unwrap();

    assert!(sig_path.exists());
    let sig_content = std::fs::read_to_string(&sig_path).unwrap();
    assert!(STANDARD.decode(sig_content.trim()).is_ok());

    cargo_codesign::update::verify_file(&archive_path, &sig_path, &public_b64).unwrap();
}
