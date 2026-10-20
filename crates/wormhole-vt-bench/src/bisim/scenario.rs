//! Scenarios from the tools' own inputs: the corpora and the `.vtt` cases.

use super::{Resize, Scenario, Setup};
use crate::corpus::Corpus;
use crate::vtt::{Case, StepKind};

/// A corpus as a scenario: its stream at its size, with `scrollback` lines of history.
pub fn from_corpus(corpus: &Corpus, scrollback: usize) -> Scenario {
    Scenario {
        name: corpus.name.to_string(),
        setup: Setup {
            cols: corpus.cols,
            rows: corpus.rows,
            scrollback,
            allow_checksum: false,
            allow_window_ops: false,
            allow_title_report: false,
        },
        bytes: corpus.bytes.clone(),
        resizes: Vec::new(),
    }
}

/// A `.vtt` case as a scenario: everything it feeds, in order, and its resizes at the offsets
/// they happen at. The checks it makes of the screen are not used: the bisimulation compares the
/// whole state instead.
pub fn from_case(case: &Case) -> Scenario {
    let mut bytes = Vec::new();
    let mut resizes = Vec::new();
    for step in &case.steps {
        match &step.kind {
            StepKind::Feed(data) => bytes.extend_from_slice(data),
            StepKind::Resize { cols, rows } => {
                resizes.push(Resize { at: bytes.len(), cols: *cols, rows: *rows })
            }
            _ => {}
        }
    }
    Scenario {
        name: format!("{}:{} {}", case.file, case.line, case.name),
        setup: Setup {
            cols: case.cols,
            rows: case.rows,
            scrollback: case.scrollback_limit,
            allow_checksum: case.allow_screen_checksum,
            allow_window_ops: case.allow_window_ops,
            allow_title_report: case.allow_title_report,
        },
        bytes,
        resizes,
    }
}
