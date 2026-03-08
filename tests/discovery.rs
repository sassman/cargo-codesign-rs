use std::path::PathBuf;

use cargo_codesign::discovery::parse_metadata;

fn sample_metadata() -> &'static str {
    r#"{
        "packages": [
            {
                "name": "my-cli",
                "version": "1.0.0",
                "targets": [
                    {
                        "name": "my-cli",
                        "kind": ["bin"],
                        "src_path": "/workspace/src/main.rs"
                    },
                    {
                        "name": "my-cli",
                        "kind": ["lib"],
                        "src_path": "/workspace/src/lib.rs"
                    }
                ]
            },
            {
                "name": "helper",
                "version": "0.1.0",
                "targets": [
                    {
                        "name": "helper",
                        "kind": ["lib"],
                        "src_path": "/workspace/crates/helper/src/lib.rs"
                    }
                ]
            },
            {
                "name": "my-gui",
                "version": "2.0.0",
                "targets": [
                    {
                        "name": "my-gui",
                        "kind": ["bin"],
                        "src_path": "/workspace/crates/gui/src/main.rs"
                    }
                ]
            }
        ],
        "target_directory": "/workspace/target",
        "workspace_root": "/workspace"
    }"#
}

#[test]
fn parse_metadata_extracts_bin_targets_only() {
    let binaries = parse_metadata(sample_metadata()).unwrap();
    assert_eq!(binaries.len(), 2);
    assert_eq!(binaries[0].name, "my-cli");
    assert_eq!(binaries[1].name, "my-gui");
}

#[test]
fn release_path_is_correct() {
    let binaries = parse_metadata(sample_metadata()).unwrap();
    let expected: PathBuf = ["/workspace", "target", "release", "my-cli"]
        .iter()
        .collect();
    assert_eq!(binaries[0].release_path(), expected);
}

#[test]
fn signed_release_path_is_correct() {
    let binaries = parse_metadata(sample_metadata()).unwrap();
    let expected: PathBuf = ["/workspace", "target", "signed", "release", "my-cli"]
        .iter()
        .collect();
    assert_eq!(binaries[0].signed_release_path(), expected);
}

#[test]
fn parse_metadata_with_no_binaries_returns_empty() {
    let json = r#"{
        "packages": [
            {
                "name": "lib-only",
                "version": "1.0.0",
                "targets": [
                    { "name": "lib-only", "kind": ["lib"], "src_path": "/src/lib.rs" }
                ]
            }
        ],
        "target_directory": "/target",
        "workspace_root": "/"
    }"#;
    let binaries = parse_metadata(json).unwrap();
    assert!(binaries.is_empty());
}

#[test]
fn parse_metadata_bad_json_returns_error() {
    let result = parse_metadata("not json");
    assert!(result.is_err());
}
