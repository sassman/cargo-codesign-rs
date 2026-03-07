use cargo_sign::config::resolve::resolve_config;
use std::fs;
use tempfile::TempDir;

#[test]
fn resolve_root_sign_toml() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("sign.toml");
    fs::write(&config_path, "[macos]\nauth = \"api-key\"\n[macos.env]\n").unwrap();

    let (config, path, warnings) = resolve_config(Some(dir.path())).unwrap();
    assert!(config.macos.is_some());
    assert_eq!(path, config_path);
    assert!(warnings.is_empty());
}

#[test]
fn resolve_dot_cargo_sign_toml_fallback() {
    let dir = TempDir::new().unwrap();
    let cargo_dir = dir.path().join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();
    let config_path = cargo_dir.join("sign.toml");
    fs::write(&config_path, "[macos]\nauth = \"api-key\"\n[macos.env]\n").unwrap();

    let (config, path, warnings) = resolve_config(Some(dir.path())).unwrap();
    assert!(config.macos.is_some());
    assert_eq!(path, config_path);
    assert!(warnings.is_empty());
}

#[test]
fn resolve_root_wins_over_dot_cargo_with_warning() {
    let dir = TempDir::new().unwrap();

    let root_path = dir.path().join("sign.toml");
    fs::write(&root_path, "[macos]\nauth = \"api-key\"\n[macos.env]\n").unwrap();

    let cargo_dir = dir.path().join(".cargo");
    fs::create_dir_all(&cargo_dir).unwrap();
    fs::write(cargo_dir.join("sign.toml"), "[windows]\n[windows.env]\n").unwrap();

    let (config, path, warnings) = resolve_config(Some(dir.path())).unwrap();
    assert_eq!(path, root_path);
    assert!(config.macos.is_some());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("both"));
}

#[test]
fn resolve_no_config_found() {
    let dir = TempDir::new().unwrap();
    let result = resolve_config(Some(dir.path()));
    assert!(result.is_err());
}

#[test]
fn resolve_explicit_path_overrides_discovery() {
    let dir = TempDir::new().unwrap();
    let custom_path = dir.path().join("custom-sign.toml");
    fs::write(
        &custom_path,
        "[update]\npublic-key = \"key.pub\"\n[update.env]\nsigning-key = \"K\"\n",
    )
    .unwrap();

    let (config, path, warnings) =
        cargo_sign::config::resolve::resolve_config_from_path(&custom_path).unwrap();
    assert!(config.update.is_some());
    assert_eq!(path, custom_path);
    assert!(warnings.is_empty());
}
