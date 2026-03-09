use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

pub fn generate_keypair() -> Result<(String, String), Box<dyn std::error::Error>> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_b64 = STANDARD.encode(signing_key.to_bytes());
    let public_b64 = STANDARD.encode(verifying_key.to_bytes());

    Ok((private_b64, public_b64))
}
