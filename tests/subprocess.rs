use cargo_sign::subprocess::run;

#[test]
fn run_echo_succeeds() {
    let result = run("echo", &["hello", "world"], false).unwrap();
    assert_eq!(result.stdout.trim(), "hello world");
    assert!(result.success);
}

#[test]
fn run_false_fails() {
    let result = run("false", &[], false).unwrap();
    assert!(!result.success);
}

#[test]
fn run_nonexistent_binary_returns_error() {
    let result = run("this-binary-does-not-exist-xyz", &[], false);
    assert!(result.is_err());
}
