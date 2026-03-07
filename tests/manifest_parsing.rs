use cargo_codesign::manifest::parse_dist_manifest;

fn sample_manifest() -> &'static str {
    r#"{
        "dist_version": "0.14.0",
        "artifacts": {
            "my-cli-x86_64-apple-darwin.tar.gz": {
                "name": "my-cli-x86_64-apple-darwin.tar.gz",
                "kind": "executable-zip",
                "target_triples": ["x86_64-apple-darwin"],
                "assets": [
                    { "name": "my-cli", "path": "my-cli-x86_64-apple-darwin/my-cli" }
                ]
            },
            "my-cli-aarch64-apple-darwin.tar.gz": {
                "name": "my-cli-aarch64-apple-darwin.tar.gz",
                "kind": "executable-zip",
                "target_triples": ["aarch64-apple-darwin"],
                "assets": [
                    { "name": "my-cli", "path": "my-cli-aarch64-apple-darwin/my-cli" }
                ]
            },
            "my-cli-x86_64-unknown-linux-gnu.tar.gz": {
                "name": "my-cli-x86_64-unknown-linux-gnu.tar.gz",
                "kind": "executable-zip",
                "target_triples": ["x86_64-unknown-linux-gnu"],
                "assets": [
                    { "name": "my-cli", "path": "my-cli-x86_64-unknown-linux-gnu/my-cli" }
                ]
            },
            "my-cli-x86_64-pc-windows-msvc.zip": {
                "name": "my-cli-x86_64-pc-windows-msvc.zip",
                "kind": "executable-zip",
                "target_triples": ["x86_64-pc-windows-msvc"],
                "assets": [
                    { "name": "my-cli.exe", "path": "my-cli-x86_64-pc-windows-msvc/my-cli.exe" }
                ]
            },
            "my-cli-installer.msi": {
                "name": "my-cli-installer.msi",
                "kind": "installer",
                "target_triples": ["x86_64-pc-windows-msvc"],
                "assets": []
            }
        }
    }"#
}

#[test]
fn parse_manifest_extracts_all_artifacts() {
    let artifacts = parse_dist_manifest(sample_manifest()).unwrap();
    assert_eq!(artifacts.len(), 5);
}

#[test]
fn filter_macos_artifacts() {
    let artifacts = parse_dist_manifest(sample_manifest()).unwrap();
    let macos: Vec<_> = artifacts.iter().filter(|a| a.is_macos()).collect();
    assert_eq!(macos.len(), 2);
}

#[test]
fn filter_windows_artifacts() {
    let artifacts = parse_dist_manifest(sample_manifest()).unwrap();
    let windows: Vec<_> = artifacts.iter().filter(|a| a.is_windows()).collect();
    assert_eq!(windows.len(), 2);
}

#[test]
fn filter_linux_artifacts() {
    let artifacts = parse_dist_manifest(sample_manifest()).unwrap();
    let linux: Vec<_> = artifacts.iter().filter(|a| a.is_linux()).collect();
    assert_eq!(linux.len(), 1);
}

#[test]
fn parse_manifest_bad_json_returns_error() {
    let result = parse_dist_manifest("not json");
    assert!(result.is_err());
}

#[test]
fn parse_manifest_empty_artifacts() {
    let json = r#"{ "dist_version": "0.14.0", "artifacts": {} }"#;
    let artifacts = parse_dist_manifest(json).unwrap();
    assert!(artifacts.is_empty());
}
