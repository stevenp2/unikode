use std::process::Command;

#[test]
fn help_and_version_are_successful_non_interactive_commands() {
    for argument in ["--help", "--version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_unikode"))
            .arg(argument)
            .output()
            .expect("run unikode");
        assert!(output.status.success(), "{argument} should succeed");
        assert!(!output.stdout.is_empty());
    }
}

#[test]
fn invalid_arguments_fail_without_starting_the_tui() {
    let output = Command::new(env!("CARGO_BIN_EXE_unikode"))
        .arg("--definitely-not-a-real-option")
        .output()
        .expect("run unikode");
    assert!(!output.status.success());
}
