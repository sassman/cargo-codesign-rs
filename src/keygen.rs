use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use std::io::Write;
use std::path::Path;

pub fn generate_keypair() -> Result<(String, String), Box<dyn std::error::Error>> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_b64 = STANDARD.encode(signing_key.to_bytes());
    let public_b64 = STANDARD.encode(verifying_key.to_bytes());

    Ok((private_b64, public_b64))
}

/// Adds the private key filename to the `.gitignore` in the same directory.
///
/// Returns `Ok(true)` if the entry was added, `Ok(false)` if it was already present.
pub fn update_gitignore(private_key_path: &Path) -> Result<bool, std::io::Error> {
    let dir = private_key_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let gitignore_path = dir.join(".gitignore");

    let key_name = private_key_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "could not determine filename from path: {}",
                    private_key_path.display()
                ),
            )
        })?;

    let existing = std::fs::read_to_string(&gitignore_path).unwrap_or_default();

    if existing.lines().any(|line| line.trim() == key_name) {
        return Ok(false);
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore_path)?;

    if !existing.is_empty() && !existing.ends_with('\n') {
        writeln!(file)?;
    }

    writeln!(file, "# private key for codesign update signing:")?;
    writeln!(file, "{key_name}")?;

    Ok(true)
}
