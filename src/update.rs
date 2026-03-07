use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum UpdateSignError {
    #[error("invalid private key: {0}")]
    InvalidPrivateKey(String),
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn sign_bytes(data: &[u8], private_key_b64: &str) -> Result<String, UpdateSignError> {
    let key_bytes = STANDARD
        .decode(private_key_b64)
        .map_err(|e| UpdateSignError::InvalidPrivateKey(e.to_string()))?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| UpdateSignError::InvalidPrivateKey("expected 32 bytes".into()))?;
    let signing_key = SigningKey::from_bytes(&key_array);
    let signature = signing_key.sign(data);
    Ok(STANDARD.encode(signature.to_bytes()))
}

pub fn verify_bytes(
    data: &[u8],
    signature_b64: &str,
    public_key_b64: &str,
) -> Result<(), UpdateSignError> {
    let pub_bytes = STANDARD
        .decode(public_key_b64)
        .map_err(|e| UpdateSignError::InvalidPublicKey(e.to_string()))?;
    let pub_array: [u8; 32] = pub_bytes
        .try_into()
        .map_err(|_| UpdateSignError::InvalidPublicKey("expected 32 bytes".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&pub_array)
        .map_err(|e| UpdateSignError::InvalidPublicKey(e.to_string()))?;

    let sig_bytes = STANDARD
        .decode(signature_b64)
        .map_err(|e| UpdateSignError::InvalidSignature(e.to_string()))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| UpdateSignError::InvalidSignature("expected 64 bytes".into()))?;
    let signature = Signature::from_bytes(&sig_array);

    verifying_key
        .verify(data, &signature)
        .map_err(|_| UpdateSignError::VerificationFailed)
}

pub fn sign_file(
    archive_path: &Path,
    sig_path: &Path,
    private_key_b64: &str,
) -> Result<(), UpdateSignError> {
    let data = std::fs::read(archive_path)?;
    let signature_b64 = sign_bytes(&data, private_key_b64)?;
    std::fs::write(sig_path, format!("{signature_b64}\n"))?;
    Ok(())
}

pub fn verify_file(
    archive_path: &Path,
    sig_path: &Path,
    public_key_b64: &str,
) -> Result<(), UpdateSignError> {
    let data = std::fs::read(archive_path)?;
    let signature_b64 = std::fs::read_to_string(sig_path)?;
    verify_bytes(&data, signature_b64.trim(), public_key_b64)
}
