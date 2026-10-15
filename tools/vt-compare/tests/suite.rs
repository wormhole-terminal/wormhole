//! The whole conformance suite on every engine this machine can run.

use std::path::Path;

use vt_compare::engines::{engine_names, for_each_engine};
use vt_compare::matrix::{Outcome, Runner, load_cases, quiet_panics};

fn matrix() -> vt_compare::matrix::Matrix {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/wormhole-vt/tests/conformance");
    let cases = load_cases(&dir).unwrap_or_else(|e| panic!("{e}"));
    quiet_panics();
    let mut runner = Runner::new(&cases);
    for_each_engine(&mut runner);
    runner.finish()
}

#[test]
fn every_engine_answers_every_case_without_crashing() {
    let matrix = matrix();
    assert!(matrix.rows.len() >= 400, "only {} cases", matrix.rows.len());
    assert_eq!(matrix.engines, engine_names());
    for (i, engine) in matrix.engines.iter().enumerate() {
        let crashes: Vec<String> = matrix
            .failures_of(i)
            .filter(|(_, f)| f.step == "panic")
            .map(|(row, f)| format!("{}:{} {}: {}", row.area, row.line, row.name, f.detail))
            .collect();
        assert!(crashes.is_empty(), "{engine} crashed on:\n{}", crashes.join("\n"));
    }
}

/// alacritty is the one engine whose failures the suite already lists (`known-fail:`), so the
/// matrix must show exactly those: an unmarked failure or a stale mark is a bug in an adapter.
#[test]
fn alacritty_fails_what_the_cases_say_it_fails() {
    let matrix = matrix();
    let Some(i) = matrix.engines.iter().position(|e| e == "alacritty") else { return };
    for (row, failure) in matrix.failures_of(i) {
        assert!(
            failure.known,
            "{}:{} {} fails without a known-fail mark: {}",
            row.area, row.line, row.name, failure.detail
        );
    }
    assert!(matrix.rows.iter().all(|r| !matches!(r.outcomes[i], Outcome::Na(_))));
}
