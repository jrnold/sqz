//! Runs the command-line tool end to end.
#![allow(clippy::unwrap_used, clippy::panic)]
use std::path::PathBuf;
use std::process::{Command, Output};

const CRETE: &str = "tests/fixtures/original/CRETE.WQ!";
const LOCKED: &str = "tests/fixtures/variants/CPW.WQ!";

fn sqz(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sqz"))
        .args(args)
        .output()
        .unwrap()
}

fn temp(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    dir.join(format!("{}-{name}", std::process::id()))
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn decode_writes_csv_with_recovered_values() {
    let out = temp("crete.csv");
    let run = sqz(&["decode", CRETE, out.to_str().unwrap()]);
    assert!(run.status.success(), "{}", stderr(&run));
    let csv = std::fs::read_to_string(out).unwrap();
    assert!(csv.starts_with(",Dataset: CRETE-1993,"));
    assert!(csv.contains("2,,,1,7400,386.63832737042276,"));
}

#[test]
fn wq1_writes_a_lotus_style_file() {
    let out = temp("crete.wq1");
    let run = sqz(&["wq1", CRETE, out.to_str().unwrap()]);
    assert!(run.status.success(), "{}", stderr(&run));
    assert_eq!(std::fs::read(out).unwrap()[..6], [0, 0, 2, 0, 0x20, 0x51]);
}

#[test]
fn info_reports_options() {
    let run = sqz(&["info", LOCKED]);
    assert!(run.status.success());
    assert!(String::from_utf8_lossy(&run.stdout).contains("password_protected: true"));
}

#[test]
fn passwords_are_accepted_and_required() {
    let out = temp("locked.csv");
    let out = out.to_str().unwrap();
    let missing = sqz(&["decode", LOCKED, out]);
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("password protected"));
    let wrong = sqz(&["decode", "--password", "nope", LOCKED, out]);
    assert!(stderr(&wrong).contains("wrong password"));
    assert!(
        sqz(&["decode", "--password", "secret1", LOCKED, out])
            .status
            .success()
    );
}

#[test]
fn files_without_results_warn_and_leave_formulas_empty() {
    let out = temp("removed.csv");
    let run = sqz(&[
        "decode",
        "tests/fixtures/variants/CRM.WQ!",
        out.to_str().unwrap(),
    ]);
    assert!(run.status.success());
    assert!(stderr(&run).contains("no stored formula results"));
    assert!(
        std::fs::read_to_string(out)
            .unwrap()
            .contains("1,,,0,8100,,,,40550,0,")
    );
}

#[test]
fn ruled_labels_stay_exact_in_csv() {
    let out = temp("rules.csv");
    assert!(
        sqz(&["decode", CRETE, out.to_str().unwrap()])
            .status
            .success()
    );
    assert!(
        std::fs::read_to_string(out)
            .unwrap()
            .contains(",+---------,")
    );
}

#[test]
fn bad_usage_and_bad_files_fail_cleanly() {
    assert!(stderr(&sqz(&[])).contains("usage:"));
    assert!(stderr(&sqz(&["decode", "--password"])).contains("usage:"));
    let run = sqz(&["info", "README.md"]);
    assert!(!run.status.success());
    assert!(stderr(&run).contains("not a supported"), "{}", stderr(&run));
}
