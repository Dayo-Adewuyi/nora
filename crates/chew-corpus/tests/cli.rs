use assert_cmd::Command;

#[test]
fn help_lists_all_pipeline_commands() {
    let mut command = Command::cargo_bin("chew-corpus-pipeline").unwrap();
    let output = command.arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for expected in ["extract", "compare", "run", "verify"] {
        assert!(
            stdout.contains(expected),
            "help omitted {expected}: {stdout}"
        );
    }
}

#[test]
fn missing_explicit_config_exits_with_usage_error() {
    let mut command = Command::cargo_bin("chew-corpus-pipeline").unwrap();
    command.arg("verify").arg("--config").assert().code(2);
}
