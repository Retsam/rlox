use core::str;
use std::process::{Command, Output};

enum FailureType {
    CompileError,
    RuntimeError,
}
use FailureType::*;
enum TestCaseResult {
    Success,
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

    let output = run_rlox(format!("./tests/examples/{file}.lox")).unwrap();
    assert_eq!(str::from_utf8(&output.stdout).unwrap(), stdout);

    match result {
        TestCaseResult::Success => assert!(output.status.success(), "Expected no error code"),
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
fn unicode_comments_test() {
    run_test(TestCase {
        file: "unicode_comments",
        stdout: "ok\n",
        result: Success,
    });
}

#[test]
fn expressions() {
    run_test(TestCase {
        file: "expressions",
        stdout: "5\n",
        result: Success,
    });
}

#[test]
fn plus_operator() {
    run_test(TestCase {
        file: "plus_operator",
        stdout: "3\nab\n",
        result: Failure(
            RuntimeError,
            "Operands must be two numbers or two strings.\n[line 3] in script\n",
        ),
    });
}

#[test]
fn strings() {
    run_test(TestCase {
        file: "strings",
        stdout: "true\ntrue\nfalse\n",
        result: Success,
    });
}

#[test]
fn assignment() {
    run_test(TestCase {
        file: "assignment",
        stdout: "1\n2\n3\n1\n",
        result: Success,
    });
}

#[test]
fn assign_error_test() {
    run_test(TestCase {
        file: "assign_error",
        stdout: "",
        result: Failure(
            FailureType::CompileError,
            "[line 1] Error at '=': Invalid assignment target.\n",
        ),
    });
}

#[test]
fn locals() {
    run_test(TestCase {
        file: "locals",
        stdout: "",
        result: Failure(
            CompileError,
            "\
[line 3] Error at 'x': Already a variable with this name in this scope.
[line 6] Error at 'x': Can't read local variable in its own initializer.\n",
        ),
    });
}

#[test]
fn logical() {
    run_test(TestCase {
        file: "logical",
        stdout: "one\nnil\nfalse\n3\nzero\nnothing\ntwo\nnil\n3\nnil\n",
        result: Success,
    });
}

#[test]
fn control_flow() {
    run_test(TestCase {
        file: "control_flow",
        stdout: "false\nalways\n0\n1\n2\n",
        result: Success,
    });
}
