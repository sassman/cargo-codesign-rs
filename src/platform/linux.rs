use crate::subprocess::{run, SubprocessError};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum LinuxSignError {
    #[error("subprocess failed: {0}")]
    Subprocess(#[from] SubprocessError),
    #[error("signing failed for {path}: {detail}")]
    SigningFailed { path: PathBuf, detail: String },
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct SignOpts<'a> {
    pub verbose: bool,
    pub output: Option<&'a Path>,
}

/// Compute the output path: use custom output if provided, else append `ext` to the archive path.
fn resolve_output(archive: &Path, ext: &str, custom: Option<&Path>) -> PathBuf {
    custom.map_or_else(
        || {
            let mut p = archive.as_os_str().to_owned();
            p.push(ext);
            PathBuf::from(p)
        },
        Path::to_path_buf,
    )
}

/// Sign an archive using cosign keyless OIDC.
pub fn sign_cosign(archive: &Path, opts: &SignOpts<'_>) -> Result<PathBuf, LinuxSignError> {
    let archive_str = archive.to_string_lossy().to_string();
    let bundle_path = resolve_output(archive, ".bundle", opts.output);
    let bundle_str = bundle_path.to_string_lossy().to_string();

    let output = run(
        "cosign",
        &["sign-blob", "--bundle", &bundle_str, &archive_str, "--yes"],
        opts.verbose,
    )?;
    if !output.success {
        return Err(LinuxSignError::SigningFailed {
            path: archive.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(bundle_path)
}

/// Sign an archive using minisign with a key from an env var.
pub fn sign_minisign(
    archive: &Path,
    key_content: &str,
    opts: &SignOpts<'_>,
) -> Result<PathBuf, LinuxSignError> {
    let archive_str = archive.to_string_lossy().to_string();
    let sig_path = resolve_output(archive, ".minisig", opts.output);
    let sig_str = sig_path.to_string_lossy().to_string();

    // Write key to temp file
    let key_dir = tempfile::TempDir::new()?;
    let key_path = key_dir.path().join("minisign.key");
    std::fs::write(&key_path, key_content)?;
    let key_str = key_path.to_string_lossy().to_string();

    let output = run(
        "minisign",
        &["-S", "-s", &key_str, "-m", &archive_str, "-x", &sig_str],
        opts.verbose,
    )?;

    // Key file is cleaned up when key_dir drops

    if !output.success {
        return Err(LinuxSignError::SigningFailed {
            path: archive.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(sig_path)
}

/// Sign an archive using GPG detached signature.
pub fn sign_gpg(archive: &Path, opts: &SignOpts<'_>) -> Result<PathBuf, LinuxSignError> {
    let archive_str = archive.to_string_lossy().to_string();
    let sig_path = resolve_output(archive, ".sig", opts.output);
    let sig_str = sig_path.to_string_lossy().to_string();

    let output = run(
        "gpg",
        &["--detach-sign", "--output", &sig_str, &archive_str],
        opts.verbose,
    )?;
    if !output.success {
        return Err(LinuxSignError::SigningFailed {
            path: archive.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(sig_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cosign_output_has_bundle_extension() {
        let result = resolve_output(Path::new("release.tar.gz"), ".bundle", None);
        assert_eq!(result, PathBuf::from("release.tar.gz.bundle"));
    }

    #[test]
    fn default_minisig_output_has_minisig_extension() {
        let result = resolve_output(Path::new("release.tar.gz"), ".minisig", None);
        assert_eq!(result, PathBuf::from("release.tar.gz.minisig"));
    }

    #[test]
    fn default_gpg_output_has_sig_extension() {
        let result = resolve_output(Path::new("release.tar.gz"), ".sig", None);
        assert_eq!(result, PathBuf::from("release.tar.gz.sig"));
    }

    #[test]
    fn custom_output_overrides_default() {
        let custom = PathBuf::from("custom.sig");
        let result = resolve_output(Path::new("release.tar.gz"), ".bundle", Some(&custom));
        assert_eq!(result, custom);
    }
}
