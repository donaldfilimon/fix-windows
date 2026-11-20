use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fix-windows"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn categories_and_help_without_mounts() {
    let out = run(&["categories"]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 12);
    assert!(run(&["audit", "--help"]).status.success());
}
#[test]
fn reject_path_escape_and_invalid_filters_before_audit() {
    for (opt, value) in [
        ("--report-name", "../secret"),
        ("--report-name", "/tmp/report"),
        ("--report-name", ".."),
        ("--category", "made-up"),
        ("--kind", "both"),
    ] {
        let out = run(&["audit", "a", "b", opt, value]);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("invalid value"));
    }
}
