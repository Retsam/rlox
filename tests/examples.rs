use core::str;
use std::process::{Command, Output};

enum FailureType {
    CompileError,
    RuntimeError,
}
enum TestCaseResult {
    Success(),
    Failure(FailureType, &'static str),
}
use TestCaseResult::*;

struct TestCase {
    file: &'static str,
    stdout: &'static str,
    result: TestCaseResult,
}

fn run_rlox(file: String) -> Result<Output, std::io::Error> {
    let mut input = Command::new("./target/debug/rlox");
    input.arg(file).output()
}

fn run_test(case: TestCase) {
    let TestCase {
        file,
        stdout,
        result,
    } = case;

    let output = run_rlox(format!("./tests/examples/{file}")).unwrap();
    assert_eq!(str::from_utf8(&output.stdout).unwrap(), stdout);

    match result {
        TestCaseResult::Success() => assert!(output.status.success(), "Expected no error code"),
        TestCaseResult::Failure(failure_type, stderr) => {
            assert_eq!(
                output.status.code().unwrap(),
                match failure_type {
                    FailureType::CompileError => 65,
                    FailureType::RuntimeError => 70,
                }
            );
            assert_eq!(str::from_utf8(&output.stderr).unwrap(), stderr);
        }
    }
}

#[test]
fn print_test() {
    run_test(TestCase {
        file: "print.lox",
        stdout: "3\n",
        result: Success(),
    });
}

#[test]
fn assign_error_test() {
    run_test(TestCase {
        file: "assign_error.lox",
        stdout: "",
        result: Failure(
            FailureType::CompileError,
            "[line 1] Error at =: Invalid assignment target.\n",
        ),
    });
}

#[test]
fn var_assignment() {
    run_test(TestCase {
        file: "var_assignment.lox",
        stdout: "1\n",
        result: Success(),
    });
}
