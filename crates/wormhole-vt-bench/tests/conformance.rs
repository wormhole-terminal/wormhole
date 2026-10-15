//! Runs the `.vtt` cases in `crates/wormhole-vt/tests/conformance/` against every engine.
//!
//! `VTT_FILTER=<text>` runs only the cases whose name contains the text; `VTT_VERBOSE=1` lists the
//! known failures of each engine. Run with `-- --nocapture` to see the report when all is well.

use std::path::{Path, PathBuf};

use wormhole_vt_bench::Engine;
use wormhole_vt_bench::engines::{ENGINE_NAMES, EngineVisitor, OTHER_ENGINE_NAMES, for_each_engine};
use wormhole_vt_bench::vtt::{self, Case};

fn suite_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../wormhole-vt/tests/conformance")
}

fn load() -> Vec<Case> {
    match vtt::load_dir(&suite_dir()) {
        Ok(cases) => cases,
        Err(errors) => panic!("the conformance files do not parse:\n{}", errors.join("\n")),
    }
}

struct RunAll<'a> {
    cases: &'a [Case],
    filter: Option<String>,
    verbose: bool,
    failures: usize,
}

impl EngineVisitor for RunAll<'_> {
    fn visit<E: Engine>(&mut self) {
        let report = vtt::run_suite::<E>(self.cases, self.filter.as_deref());
        eprintln!("{}", report.render(self.verbose));
        self.failures += report.failed.len();
    }
}

#[test]
fn every_engine_passes_what_it_is_not_marked_to_fail() {
    let cases = load();
    let mut run = RunAll {
        cases: &cases,
        filter: std::env::var("VTT_FILTER").ok().filter(|f| !f.is_empty()),
        verbose: std::env::var_os("VTT_VERBOSE").is_some(),
        failures: 0,
    };
    for_each_engine(&mut run);
    assert_eq!(run.failures, 0, "{} conformance cases failed (details above)", run.failures);
}

#[test]
fn the_suite_is_large_and_well_formed() {
    let cases = load();
    assert!(cases.len() >= 200, "only {} cases", cases.len());
    for case in &cases {
        for known in &case.known_fail {
            assert!(
                ENGINE_NAMES.contains(&known.engine.as_str())
                    || OTHER_ENGINE_NAMES.contains(&known.engine.as_str()),
                "{}:{} {:?}: known-fail names {:?}, which is not an engine ({ENGINE_NAMES:?} or {OTHER_ENGINE_NAMES:?})",
                case.file,
                case.line,
                case.name,
                known.engine
            );
        }
        assert!(
            case.basis.is_some(),
            "{}:{} {:?} has no `spec` or `convention` line",
            case.file,
            case.line,
            case.name
        );
    }
}
