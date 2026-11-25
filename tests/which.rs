use std::process::{Command, Output};

fn which() -> Command {
    Command::new(env!("CARGO_BIN_EXE_which"))
}

fn run(args: &[&str]) -> Output {
    which()
        .args(args)
        .output()
        .expect("failed to execute which")
}

#[test]
fn help_returns_success() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage"));
}

#[test]
fn short_help_returns_success() {
    let output = run(&["-h"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage"));
}

#[test]
fn version_returns_success() {
    let output = run(&["--version"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("which"));
}

#[test]
fn short_version_returns_success() {
    let output = run(&["-V"]);

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("which"));
}

#[test]
fn no_arguments_returns_exit_code_2() {
    let output = run(&[]);

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn nonexistent_command_returns_exit_code_1() {
    let output = run(&["this-command-should-not-exist-123456"]);

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn silent_mode_does_not_print_missing_command() {
    let output = run(&["--silent", "this-command-should-not-exist-123456"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn short_silent_mode_does_not_print_missing_command() {
    let output = run(&["-s", "this-command-should-not-exist-123456"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}

#[test]
fn unknown_option_returns_exit_code_2() {
    let output = run(&["--unknown-option"]);

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn finds_existing_command() {
    let output = run(&["sh"]);

    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn all_option_returns_success_for_existing_command() {
    let output = run(&["--all", "sh"]);

    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn multiple_commands_are_supported() {
    let output = run(&["sh", "this-command-should-not-exist-123456"]);

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sh"));
}

#[test]
fn end_of_options_allows_dash_prefixed_argument() {
    let output = run(&["--", "--not-a-real-command"]);

    assert_eq!(output.status.code(), Some(1));
}
