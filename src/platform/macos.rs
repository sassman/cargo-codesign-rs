use crate::subprocess::{run, SubprocessError};
use rand_core::RngCore;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum MacosSignError {
    #[error("subprocess failed: {0}")]
    Subprocess(#[from] SubprocessError),
    #[error("codesign failed for {path}: {detail}")]
    CodesignFailed { path: PathBuf, detail: String },
    #[error("DMG creation failed: {0}")]
    DmgCreationFailed(String),
    #[error("notarization failed: {0}")]
    NotarizationFailed(String),
    #[error("stapling failed: {0}")]
    StaplingFailed(String),
    #[error("keychain operation failed: {0}")]
    KeychainFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct CodesignOpts<'a> {
    pub identity: &'a str,
    pub entitlements: Option<&'a Path>,
    pub verbose: bool,
}

/// Sign a single binary or bundle with `codesign`.
pub fn codesign(path: &Path, opts: &CodesignOpts<'_>) -> Result<(), MacosSignError> {
    let mut args = vec![
        "--force",
        "--timestamp",
        "--options",
        "runtime",
        "--sign",
        opts.identity,
    ];

    let entitlements_str;
    if let Some(e) = opts.entitlements {
        entitlements_str = e.to_string_lossy().to_string();
        args.push("--entitlements");
        args.push(&entitlements_str);
    }

    let path_str = path.to_string_lossy().to_string();
    args.push(&path_str);

    let output = run("codesign", &args, opts.verbose)?;
    if !output.success {
        return Err(MacosSignError::CodesignFailed {
            path: path.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(())
}

/// Sign all binaries inside a `.app` bundle, then sign the bundle itself.
pub fn codesign_app(app_path: &Path, opts: &CodesignOpts<'_>) -> Result<(), MacosSignError> {
    // Sign inner binaries in Contents/MacOS/
    let macos_dir = app_path.join("Contents/MacOS");
    if macos_dir.exists() {
        for entry in std::fs::read_dir(&macos_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let inner_opts = CodesignOpts {
                    identity: opts.identity,
                    entitlements: None,
                    verbose: opts.verbose,
                };
                codesign(&entry.path(), &inner_opts)?;
            }
        }
    }

    // Sign frameworks in Contents/Frameworks/
    let frameworks_dir = app_path.join("Contents/Frameworks");
    if frameworks_dir.exists() {
        for entry in std::fs::read_dir(&frameworks_dir)? {
            let entry = entry?;
            let inner_opts = CodesignOpts {
                identity: opts.identity,
                entitlements: None,
                verbose: opts.verbose,
            };
            codesign(&entry.path(), &inner_opts)?;
        }
    }

    // Sign the .app bundle itself (with entitlements)
    codesign(app_path, opts)
}

/// Create a DMG from a `.app` bundle using `hdiutil`.
///
/// Stages the `.app` alongside an `/Applications` symlink in a temporary
/// directory so the resulting DMG shows the standard drag-to-install layout.
///
/// When `dmg_config` is provided, a background image and a generated
/// `.DS_Store` (with icon positions and window properties) are placed in
/// the staging directory before creating the compressed UDZO DMG.
pub fn create_dmg(
    app_path: &Path,
    dmg_path: &Path,
    volume_name: &str,
    dmg_config: Option<&crate::config::DmgConfig>,
    verbose: bool,
) -> Result<(), MacosSignError> {
    let app_name = app_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("App.app"));

    let staging_dir = tempfile::tempdir().map_err(MacosSignError::Io)?;

    let staged_app = staging_dir.path().join(app_name);

    // Copy the .app bundle into the staging directory
    copy_dir_all(app_path, &staged_app)?;

    // Create an /Applications symlink so the DMG shows the drag-to-install target
    std::os::unix::fs::symlink("/Applications", staging_dir.path().join("Applications"))
        .map_err(MacosSignError::Io)?;

    let staging_str = staging_dir.path().to_string_lossy().to_string();
    let dmg_str = dmg_path.to_string_lossy().to_string();

    match dmg_config {
        Some(cfg) => create_dmg_styled(&staging_str, &dmg_str, volume_name, app_name, cfg, verbose),
        None => create_dmg_plain(&staging_str, &dmg_str, volume_name, verbose),
    }
}

/// Plain DMG: single hdiutil call, no background or icon positioning.
fn create_dmg_plain(
    staging_str: &str,
    dmg_str: &str,
    volume_name: &str,
    verbose: bool,
) -> Result<(), MacosSignError> {
    let output = run(
        "hdiutil",
        &[
            "create",
            "-volname",
            volume_name,
            "-srcfolder",
            staging_str,
            "-ov",
            "-format",
            "UDZO",
            dmg_str,
        ],
        verbose,
    )?;
    if !output.success {
        return Err(MacosSignError::DmgCreationFailed(output.stderr));
    }
    Ok(())
}

/// Styled DMG: stage background image and a generated `.DS_Store` in the
/// staging directory, then create a compressed UDZO DMG in one step.
///
/// This avoids the flaky mount → `AppleScript` → detach → convert pipeline
/// by writing the `.DS_Store` directly with [`crate::ds_store::write_ds_store`].
fn create_dmg_styled(
    staging_str: &str,
    dmg_str: &str,
    volume_name: &str,
    app_name: &std::ffi::OsStr,
    cfg: &crate::config::DmgConfig,
    verbose: bool,
) -> Result<(), MacosSignError> {
    // Resolve background image path relative to cwd
    let bg_path = std::env::current_dir()
        .map_err(MacosSignError::Io)?
        .join(&cfg.background);
    if !bg_path.exists() {
        return Err(MacosSignError::DmgCreationFailed(format!(
            "background image not found: {}",
            bg_path.display()
        )));
    }
    // Always copy the background image as the canonical name so the alias
    // and bookmark inside the .DS_Store match the file on disk exactly.
    let bg_canonical = crate::ds_store::DMG_BG_FILENAME;

    let staging = PathBuf::from(staging_str);
    let bg_dir = staging.join(".background");
    std::fs::create_dir_all(&bg_dir).map_err(MacosSignError::Io)?;
    std::fs::copy(&bg_path, bg_dir.join(bg_canonical)).map_err(MacosSignError::Io)?;

    // Generate .DS_Store with icon positions and window properties
    let layout = crate::ds_store::DmgLayout {
        window_width: cfg.window_size[0],
        window_height: cfg.window_size[1],
        icon_size: cfg.icon_size,
        app_name: app_name.to_string_lossy().into_owned(),
        app_x: cfg.app_position[0],
        app_y: cfg.app_position[1],
        apps_link_x: cfg.app_drop_link[0],
        apps_link_y: cfg.app_drop_link[1],
        background_filename: bg_canonical.to_string(),
        volume_name: volume_name.to_string(),
    };
    let ds_store_bytes = crate::ds_store::write_ds_store(&layout);
    std::fs::write(staging.join(".DS_Store"), &ds_store_bytes).map_err(MacosSignError::Io)?;

    if verbose {
        eprintln!(
            "cargo-codesign: wrote .DS_Store ({} bytes) to staging dir",
            ds_store_bytes.len()
        );
    }

    // Create compressed UDZO DMG directly from the staged directory
    let output = run(
        "hdiutil",
        &[
            "create",
            "-volname",
            volume_name,
            "-srcfolder",
            staging_str,
            "-ov",
            "-format",
            "UDZO",
            "-imagekey",
            "zlib-level=9",
            dmg_str,
        ],
        verbose,
    )?;
    if !output.success {
        return Err(MacosSignError::DmgCreationFailed(output.stderr));
    }

    Ok(())
}

/// Recursively copy a directory tree.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), MacosSignError> {
    std::fs::create_dir_all(dst).map_err(MacosSignError::Io)?;
    for entry in std::fs::read_dir(src).map_err(MacosSignError::Io)? {
        let entry = entry.map_err(MacosSignError::Io)?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().map_err(MacosSignError::Io)?.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path).map_err(MacosSignError::Io)?;
        }
    }
    Ok(())
}

/// Codesign a DMG file.
pub fn codesign_dmg(dmg_path: &Path, opts: &CodesignOpts<'_>) -> Result<(), MacosSignError> {
    let no_entitlements_opts = CodesignOpts {
        identity: opts.identity,
        entitlements: None,
        verbose: opts.verbose,
    };
    codesign(dmg_path, &no_entitlements_opts)
}

/// Notarize an artifact using App Store Connect API key (CI mode).
pub fn notarize_api_key(
    artifact: &Path,
    key_path: &Path,
    key_id: &str,
    issuer_id: &str,
    verbose: bool,
) -> Result<(), MacosSignError> {
    let artifact_str = artifact.to_string_lossy().to_string();
    let key_str = key_path.to_string_lossy().to_string();

    let output = run(
        "xcrun",
        &[
            "notarytool",
            "submit",
            &artifact_str,
            "--wait",
            "--key",
            &key_str,
            "--key-id",
            key_id,
            "--issuer",
            issuer_id,
        ],
        verbose,
    )?;
    if !output.success {
        return Err(MacosSignError::NotarizationFailed(format!(
            "stdout: {}\nstderr: {}",
            output.stdout, output.stderr
        )));
    }
    Ok(())
}

/// Notarize an artifact using Apple ID (local/indie mode).
pub fn notarize_apple_id(
    artifact: &Path,
    apple_id: &str,
    team_id: &str,
    password: &str,
    verbose: bool,
) -> Result<(), MacosSignError> {
    let artifact_str = artifact.to_string_lossy().to_string();

    let output = run(
        "xcrun",
        &[
            "notarytool",
            "submit",
            &artifact_str,
            "--wait",
            "--apple-id",
            apple_id,
            "--team-id",
            team_id,
            "--password",
            password,
        ],
        verbose,
    )?;
    if !output.success {
        return Err(MacosSignError::NotarizationFailed(format!(
            "stdout: {}\nstderr: {}",
            output.stdout, output.stderr
        )));
    }
    Ok(())
}

/// Staple the notarization ticket to an artifact.
pub fn staple(artifact: &Path, verbose: bool) -> Result<(), MacosSignError> {
    let artifact_str = artifact.to_string_lossy().to_string();

    let output = run("xcrun", &["stapler", "staple", &artifact_str], verbose)?;
    if !output.success {
        return Err(MacosSignError::StaplingFailed(output.stderr));
    }
    Ok(())
}

/// Decode a base64-encoded `.p12` certificate to a temp file.
/// Returns the path to the temp `.p12` file.
pub fn decode_cert_to_tempfile(
    cert_base64: &str,
    dir: &std::path::Path,
) -> Result<PathBuf, MacosSignError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cert_base64.trim())
        .map_err(|e| MacosSignError::KeychainFailed(format!("base64 decode failed: {e}")))?;

    let p12_path = dir.join("cargo-codesign-cert.p12");
    std::fs::write(&p12_path, &bytes)
        .map_err(|e| MacosSignError::KeychainFailed(format!("failed to write temp .p12: {e}")))?;

    Ok(p12_path)
}

const KEYCHAIN_STATE_FILE: &str = ".codesign-keychain";

/// Derive the keychain state file path from the project root.
pub fn keychain_state_path() -> PathBuf {
    PathBuf::from("target").join(KEYCHAIN_STATE_FILE)
}

/// Persist keychain name so `--ci-cleanup-cert` can find it later.
pub fn save_keychain_state(path: &Path, keychain_name: &str) -> Result<(), MacosSignError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MacosSignError::KeychainFailed(format!("failed to create target dir: {e}"))
        })?;
    }
    std::fs::write(path, keychain_name).map_err(|e| {
        MacosSignError::KeychainFailed(format!("failed to write keychain state: {e}"))
    })?;
    Ok(())
}

/// Load the keychain name from a previous `--ci-import-cert` run.
pub fn load_keychain_state(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Import a `.p12` certificate into an ephemeral keychain (for CI api-key mode).
/// Returns the keychain path for later cleanup.
pub fn import_certificate(
    cert_p12_path: &Path,
    cert_password: &str,
    verbose: bool,
) -> Result<PathBuf, MacosSignError> {
    let random_suffix: u64 = rand_core::OsRng.next_u64();
    let keychain_name = format!("cargo-sign-{random_suffix}.keychain");
    let keychain_password = format!("{random_suffix}");

    let cert_str = cert_p12_path.to_string_lossy().to_string();

    // Create ephemeral keychain
    let output = run(
        "security",
        &["create-keychain", "-p", &keychain_password, &keychain_name],
        verbose,
    )?;
    if !output.success {
        return Err(MacosSignError::KeychainFailed(format!(
            "create-keychain failed: {}",
            output.stderr
        )));
    }

    // Import certificate
    let output = run(
        "security",
        &[
            "import",
            &cert_str,
            "-k",
            &keychain_name,
            "-P",
            cert_password,
            "-T",
            "/usr/bin/codesign",
        ],
        verbose,
    )?;
    if !output.success {
        // Cleanup on failure
        let _ = run("security", &["delete-keychain", &keychain_name], false);
        return Err(MacosSignError::KeychainFailed(format!(
            "import failed: {}",
            output.stderr
        )));
    }

    // Set key partition list so codesign can access the key
    let output = run(
        "security",
        &[
            "set-key-partition-list",
            "-S",
            "apple-tool:,apple:,codesign:",
            "-s",
            "-k",
            &keychain_password,
            &keychain_name,
        ],
        verbose,
    )?;
    if !output.success {
        let _ = run("security", &["delete-keychain", &keychain_name], false);
        return Err(MacosSignError::KeychainFailed(format!(
            "set-key-partition-list failed: {}",
            output.stderr
        )));
    }

    Ok(PathBuf::from(keychain_name))
}

/// Verify a macOS artifact's code signature via `codesign --verify`.
pub fn verify_codesign(path: &Path, verbose: bool) -> Result<(), MacosSignError> {
    let path_str = path.to_string_lossy().to_string();
    let output = run(
        "codesign",
        &["--verify", "--deep", "--strict", "-vvv", &path_str],
        verbose,
    )?;
    if !output.success {
        return Err(MacosSignError::CodesignFailed {
            path: path.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(())
}

/// Verify a macOS artifact passes Gatekeeper via `spctl --assess`.
///
/// Uses `--type open --context context:primary-signature` for `.dmg` disk images,
/// and `--type execute` for `.app` bundles and bare binaries.
pub fn verify_gatekeeper(path: &Path, verbose: bool) -> Result<(), MacosSignError> {
    let path_str = path.to_string_lossy().to_string();
    let is_dmg = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("dmg"));
    let args: Vec<&str> = if is_dmg {
        vec![
            "--assess",
            "--type",
            "open",
            "--context",
            "context:primary-signature",
            "-vvv",
            &path_str,
        ]
    } else {
        vec!["--assess", "--type", "execute", "-vvv", &path_str]
    };
    let output = run("spctl", &args, verbose)?;
    if !output.success {
        return Err(MacosSignError::CodesignFailed {
            path: path.to_path_buf(),
            detail: output.stderr,
        });
    }
    Ok(())
}

/// Delete an ephemeral keychain created by `import_certificate`.
pub fn delete_keychain(keychain_path: &Path, verbose: bool) -> Result<(), MacosSignError> {
    let keychain_str = keychain_path.to_string_lossy().to_string();
    let output = run("security", &["delete-keychain", &keychain_str], verbose)?;
    if !output.success {
        return Err(MacosSignError::KeychainFailed(format!(
            "delete-keychain failed: {}",
            output.stderr
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_cert_to_tempfile_writes_decoded_bytes() {
        use base64::Engine;
        let fake_p12 = b"fake-p12-content";
        let b64 = base64::engine::general_purpose::STANDARD.encode(fake_p12);

        let dir = tempfile::TempDir::new().unwrap();
        let p12_path = decode_cert_to_tempfile(&b64, dir.path()).unwrap();

        assert!(p12_path.exists());
        assert_eq!(std::fs::read(&p12_path).unwrap(), fake_p12);
    }

    #[test]
    fn decode_cert_to_tempfile_rejects_invalid_base64() {
        let dir = tempfile::TempDir::new().unwrap();
        let result = decode_cert_to_tempfile("not-valid-base64!!!", dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn keychain_state_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let state_path = dir.path().join(".codesign-keychain");

        save_keychain_state(&state_path, "cargo-sign-12345.keychain").unwrap();
        let loaded = load_keychain_state(&state_path).unwrap();
        assert_eq!(loaded, "cargo-sign-12345.keychain");
    }

    #[test]
    fn load_keychain_state_returns_none_when_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let state_path = dir.path().join(".codesign-keychain");
        let result = load_keychain_state(&state_path);
        assert!(result.is_none());
    }
}
