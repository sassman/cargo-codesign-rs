use crate::subprocess::{run, run_args, Arg, SubprocessError};
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
    /// Absolute path to a specific keychain to resolve `identity` from.
    ///
    /// When `Some`, `--keychain <path>` is appended to every `codesign`
    /// invocation so the binary does not have to walk the user's keychain
    /// search list. This is what makes ephemeral CI keychains work without
    /// mutating global state via `security list-keychains -d user -s`.
    pub keychain: Option<&'a Path>,
    pub verbose: bool,
}

/// Sign a single binary or bundle with `codesign`.
pub fn codesign(path: &Path, opts: &CodesignOpts<'_>) -> Result<(), MacosSignError> {
    let mut args: Vec<&str> = vec![
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

    let keychain_str;
    if let Some(k) = opts.keychain {
        keychain_str = k.to_string_lossy().to_string();
        args.push("--keychain");
        args.push(&keychain_str);
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
                    keychain: opts.keychain,
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
                keychain: opts.keychain,
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
/// by writing the `.DS_Store` directly with [`crate::ds_store::DsStoreBuilder`].
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
    let ds_store = crate::ds_store::DsStoreBuilder::new(app_name.to_string_lossy(), volume_name)
        .window_size(cfg.window_size[0], cfg.window_size[1])
        .icon_size(cfg.icon_size)
        .app_position(cfg.app_position[0], cfg.app_position[1])
        .apps_link_position(cfg.app_drop_link[0], cfg.app_drop_link[1])
        .build();
    let ds_store_bytes = ds_store.encode();
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
        keychain: opts.keychain,
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
///
/// The state file holds the absolute path to the ephemeral keychain that
/// `--ci-import-cert` created, so `--ci-cleanup-cert` and the sign step can
/// resolve it without depending on the user's keychain search list.
pub fn keychain_state_path() -> PathBuf {
    PathBuf::from("target").join(KEYCHAIN_STATE_FILE)
}

/// Persist the absolute keychain path so subsequent commands
/// (`--ci-cleanup-cert`, the sign step) can resolve the keychain without
/// touching the user's keychain search list.
pub fn save_keychain_state(state_path: &Path, keychain_path: &Path) -> Result<(), MacosSignError> {
    if let Some(parent) = state_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MacosSignError::KeychainFailed(format!("failed to create target dir: {e}"))
        })?;
    }
    std::fs::write(state_path, keychain_path.to_string_lossy().as_bytes()).map_err(|e| {
        MacosSignError::KeychainFailed(format!("failed to write keychain state: {e}"))
    })?;
    Ok(())
}

/// Load the absolute keychain path persisted by a previous
/// `--ci-import-cert` run, or `None` if the state file is missing or empty.
pub fn load_keychain_state(state_path: &Path) -> Option<PathBuf> {
    std::fs::read_to_string(state_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Pick a directory to host the ephemeral keychain.
///
/// Preference: `$RUNNER_TEMP` (set by GitHub Actions runners) → the standard
/// macOS user keychain dir `~/Library/Keychains/` → `$TMPDIR` → `/tmp`. The
/// chosen directory is the one that exists and is writable; the keychain
/// always lives at an absolute path so `security` invocations don't need the
/// keychain to be on the user's search list.
fn keychain_host_dir() -> PathBuf {
    if let Some(runner_temp) = std::env::var_os("RUNNER_TEMP") {
        let p = PathBuf::from(runner_temp);
        if p.is_dir() {
            return p;
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        let p = PathBuf::from(home).join("Library").join("Keychains");
        if p.is_dir() {
            return p;
        }
    }
    if let Some(tmp) = std::env::var_os("TMPDIR") {
        return PathBuf::from(tmp);
    }
    PathBuf::from("/tmp")
}

/// Generate an absolute keychain path and an independent password for
/// ephemeral CI keychains.
fn generate_keychain_credentials() -> (PathBuf, String) {
    let name_suffix: u64 = rand_core::OsRng.next_u64();
    // The `-db` suffix matches what `security create-keychain <name>` would
    // auto-append when given a bare name, keeping the on-disk artifact
    // recognizable to any tool that walks the directory.
    let keychain_file = format!("cargo-codesign-{name_suffix}.keychain-db");
    let keychain_path = keychain_host_dir().join(keychain_file);
    // Use a separate random value so the password is not derivable from the path.
    let keychain_password = format!("{}", rand_core::OsRng.next_u64());
    (keychain_path, keychain_password)
}

/// Import a `.p12` certificate into an ephemeral keychain (for CI api-key mode).
///
/// Creates the keychain at an absolute path so every subsequent `security`
/// and `codesign` call can address it directly via that path. The keychain
/// is intentionally **not** added to the user's keychain search list — the
/// caller is expected to pass the returned path to [`CodesignOpts::keychain`]
/// so `codesign --keychain <path>` can resolve the identity.
///
/// Returns the absolute path to the created keychain for later cleanup.
pub fn import_certificate(
    cert_p12_path: &Path,
    cert_password: &str,
    verbose: bool,
) -> Result<PathBuf, MacosSignError> {
    let (keychain_path, keychain_password) = generate_keychain_credentials();

    let cert_str = cert_p12_path.to_string_lossy().to_string();
    let keychain_str = keychain_path.to_string_lossy().to_string();

    // Create ephemeral keychain at an absolute path
    let output = run_args(
        "security",
        &[
            "create-keychain".into(),
            "-p".into(),
            Arg::sensitive(&keychain_password),
            keychain_str.as_str().into(),
        ],
        verbose,
    )?;
    if !output.success {
        return Err(MacosSignError::KeychainFailed(format!(
            "create-keychain failed: {}",
            output.stderr
        )));
    }

    // Disable auto-lock (default is 300s which is too short for CI builds)
    let output = run_args(
        "security",
        &[
            "set-keychain-settings".into(),
            "-lut".into(),
            "3600".into(),
            keychain_str.as_str().into(),
        ],
        verbose,
    )?;
    if !output.success {
        let _ = run("security", &["delete-keychain", &keychain_str], false);
        return Err(MacosSignError::KeychainFailed(format!(
            "set-keychain-settings failed: {}",
            output.stderr
        )));
    }

    // Defensively unlock the keychain. `create-keychain -p` leaves the
    // keychain unlocked, but this protects against an auto-lock racing the
    // import on slow CI runners.
    let output = run_args(
        "security",
        &[
            "unlock-keychain".into(),
            "-p".into(),
            Arg::sensitive(&keychain_password),
            keychain_str.as_str().into(),
        ],
        verbose,
    )?;
    if !output.success {
        let _ = run("security", &["delete-keychain", &keychain_str], false);
        return Err(MacosSignError::KeychainFailed(format!(
            "unlock-keychain failed: {}",
            output.stderr
        )));
    }

    // Import certificate
    let output = run_args(
        "security",
        &[
            "import".into(),
            cert_str.as_str().into(),
            "-k".into(),
            keychain_str.as_str().into(),
            "-P".into(),
            Arg::sensitive(cert_password),
            "-T".into(),
            "/usr/bin/codesign".into(),
        ],
        verbose,
    )?;
    if !output.success {
        // Cleanup on failure
        let _ = run("security", &["delete-keychain", &keychain_str], false);
        return Err(MacosSignError::KeychainFailed(format!(
            "import failed: {}",
            output.stderr
        )));
    }

    // Set key partition list so codesign can access the key
    let output = run_args(
        "security",
        &[
            "set-key-partition-list".into(),
            "-S".into(),
            "apple-tool:,apple:,codesign:".into(),
            "-s".into(),
            "-k".into(),
            Arg::sensitive(&keychain_password),
            keychain_str.as_str().into(),
        ],
        verbose,
    )?;
    if !output.success {
        let _ = run("security", &["delete-keychain", &keychain_str], false);
        return Err(MacosSignError::KeychainFailed(format!(
            "set-key-partition-list failed: {}",
            output.stderr
        )));
    }

    Ok(keychain_path)
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
    fn keychain_state_roundtrip_persists_absolute_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let state_path = dir.path().join(".codesign-keychain");
        let keychain_path = dir.path().join("cargo-codesign-42.keychain-db");

        save_keychain_state(&state_path, &keychain_path).unwrap();
        let loaded = load_keychain_state(&state_path).expect("state should load");
        assert_eq!(loaded, keychain_path);
        assert!(loaded.is_absolute());
    }

    #[test]
    fn load_keychain_state_returns_none_when_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let state_path = dir.path().join(".codesign-keychain");
        let result = load_keychain_state(&state_path);
        assert!(result.is_none());
    }

    #[test]
    fn load_keychain_state_returns_none_when_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let state_path = dir.path().join(".codesign-keychain");
        std::fs::write(&state_path, "   \n").unwrap();
        assert!(load_keychain_state(&state_path).is_none());
    }

    #[test]
    fn keychain_credentials_yield_absolute_path_and_independent_password() {
        let (path, password) = generate_keychain_credentials();

        assert!(
            path.is_absolute(),
            "keychain path must be absolute, got {}",
            path.display()
        );

        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .expect("keychain path must have a file name");
        assert!(file_name.starts_with("cargo-codesign-"));
        assert!(file_name.ends_with(".keychain-db"));

        let suffix = file_name
            .strip_prefix("cargo-codesign-")
            .unwrap()
            .strip_suffix(".keychain-db")
            .unwrap();
        assert_ne!(
            suffix, password,
            "keychain password must not equal the path's random suffix"
        );
    }

    fn build_codesign_args<'a>(
        path: &'a Path,
        opts: &'a CodesignOpts<'a>,
        entitlements_buf: &'a mut String,
        keychain_buf: &'a mut String,
        path_buf: &'a mut String,
    ) -> Vec<&'a str> {
        let mut args: Vec<&str> = vec![
            "--force",
            "--timestamp",
            "--options",
            "runtime",
            "--sign",
            opts.identity,
        ];

        if let Some(e) = opts.entitlements {
            *entitlements_buf = e.to_string_lossy().to_string();
            args.push("--entitlements");
            args.push(entitlements_buf.as_str());
        }

        if let Some(k) = opts.keychain {
            *keychain_buf = k.to_string_lossy().to_string();
            args.push("--keychain");
            args.push(keychain_buf.as_str());
        }

        *path_buf = path.to_string_lossy().to_string();
        args.push(path_buf.as_str());
        args
    }

    #[test]
    fn codesign_args_include_keychain_when_set() {
        let path = Path::new("/tmp/MyApp.app");
        let keychain = PathBuf::from("/tmp/cargo-codesign-1.keychain-db");
        let opts = CodesignOpts {
            identity: "Developer ID Application",
            entitlements: None,
            keychain: Some(&keychain),
            verbose: false,
        };

        let (mut e, mut k, mut p) = (String::new(), String::new(), String::new());
        let args = build_codesign_args(path, &opts, &mut e, &mut k, &mut p);

        let kc_idx = args
            .iter()
            .position(|a| *a == "--keychain")
            .expect("--keychain must be present");
        assert_eq!(args[kc_idx + 1], "/tmp/cargo-codesign-1.keychain-db");
        // --keychain must come before the trailing path positional
        let path_idx = args.iter().rposition(|a| *a == "/tmp/MyApp.app").unwrap();
        assert!(kc_idx + 1 < path_idx);
    }

    #[test]
    fn codesign_args_omit_keychain_when_none() {
        let path = Path::new("/tmp/MyApp.app");
        let opts = CodesignOpts {
            identity: "Developer ID Application",
            entitlements: None,
            keychain: None,
            verbose: false,
        };

        let (mut e, mut k, mut p) = (String::new(), String::new(), String::new());
        let args = build_codesign_args(path, &opts, &mut e, &mut k, &mut p);

        assert!(
            !args.contains(&"--keychain"),
            "--keychain must not appear when keychain is None: {args:?}"
        );
    }
}
