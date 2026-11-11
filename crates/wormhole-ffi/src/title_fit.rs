//! How much of the title row fits (`wormhole_core::view::layout::title_row::fit_row`): the shell
//! measures the row's texts in its own fonts, hands the widths over and draws what comes back. The
//! order in which a narrow row gives way (path, words, extras, chip, buttons, name) is core's; no
//! callback crosses the seam, so measuring stays on the Swift side.

use wormhole_core::view::layout::title_row::{Chip, Extra, Extras, PathShown, RowWants, fit_row};

/// The extras a row can carry, in the order `extras` lists them.
const EXTRAS: [Extra; 4] = [Extra::BranchStatus, Extra::Gauge, Extra::Ports, Extra::PullRequest];

/// What the row wants, each part as wide as it likes to be (widths in points, 0 or `None` for a
/// part the row does not have).
#[derive(Debug, Clone, uniffi::Record)]
pub struct TitleRowWants {
    /// The path's ways of being drawn (`TitleSnapshot.pathSteps`), widest first, each measured with
    /// the gap after the folder icon. Empty without a folder.
    pub path: Vec<f32>,
    /// The status words with their gap, 0 for none.
    pub words: f32,
    /// The branch chip whole and as its glyph alone; `None` without a branch.
    pub chip_natural: Option<f32>,
    pub chip_icon: Option<f32>,
    /// Branch status, agent gauge, ports and pull request, in that order, `None` for one the row
    /// does not have (exactly four entries).
    pub extras: Vec<Option<f32>>,
    /// The ports with one chip fewer at each step, widest first.
    pub ports_narrower: Vec<f32>,
    /// Open in… and the splits, and the one `⋯` button they fold into.
    pub group: f32,
    pub folded: f32,
    /// The terminal's name as it likes to be, and the least worth showing.
    pub name: f32,
    pub name_min: f32,
    /// Everything that never gives way.
    pub fixed: f32,
}

/// How the path is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TitlePathShown {
    None,
    /// The folder's icon alone.
    Icon,
    /// This step of `TitleRowWants.path`.
    Step {
        index: u32,
    },
}

/// What the row draws.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct TitleRowFit {
    pub path: TitlePathShown,
    pub words: bool,
    /// The chip's width (its icon width when it shrank to the glyph); `None` without a branch.
    pub chip: Option<f32>,
    /// The extras that are drawn, in the order of `TitleRowWants.extras`.
    pub extras: Vec<Option<f32>>,
    /// How many of `portsNarrower`'s steps the ports took.
    pub ports_step: u32,
    /// Open in… and the splits are one `⋯` menu.
    pub folded: bool,
    pub name: f32,
}

/// The row's fit in `avail` points.
#[uniffi::export]
pub fn title_fit_row(avail: f32, wants: TitleRowWants) -> TitleRowFit {
    let extras = EXTRAS
        .iter()
        .zip(wants.extras.iter().chain(std::iter::repeat(&None)))
        .fold(Extras::default(), |extras, (&extra, &width)| extras.with(extra, width));
    let fit = fit_row(
        avail,
        &RowWants {
            path: wants.path,
            words: wants.words,
            chip: wants
                .chip_natural
                .map(|natural| Chip { natural, icon: wants.chip_icon.unwrap_or(natural) }),
            extras,
            ports_narrower: wants.ports_narrower,
            group: wants.group,
            folded: wants.folded,
            name: wants.name,
            name_min: wants.name_min,
            fixed: wants.fixed,
        },
    );
    TitleRowFit {
        path: match fit.path {
            PathShown::None => TitlePathShown::None,
            PathShown::Icon => TitlePathShown::Icon,
            PathShown::Step(index) => TitlePathShown::Step { index: index as u32 },
        },
        words: fit.words,
        chip: fit.chip,
        extras: EXTRAS.iter().map(|&extra| fit.extras.width(extra)).collect(),
        ports_step: fit.ports_step as u32,
        folded: fit.folded,
        name: fit.name,
    }
}

#[cfg(test)]
mod tests {
    use super::{TitlePathShown, TitleRowWants, title_fit_row};

    fn wants() -> TitleRowWants {
        TitleRowWants {
            path: vec![200.0, 150.0, 90.0],
            words: 80.0,
            chip_natural: Some(70.0),
            chip_icon: Some(20.0),
            extras: vec![Some(30.0), None, Some(60.0), Some(50.0)],
            ports_narrower: vec![40.0],
            group: 60.0,
            folded: 24.0,
            name: 100.0,
            name_min: 30.0,
            fixed: 100.0,
        }
    }

    #[test]
    fn a_wide_row_keeps_everything_and_a_narrow_one_gives_way_in_core_s_order() {
        let wide = title_fit_row(1000.0, wants());
        assert_eq!((wide.path, wide.words, wide.folded), (TitlePathShown::Step { index: 0 }, true, false));
        assert_eq!(wide.extras, [Some(30.0), None, Some(60.0), Some(50.0)]);
        // 640 = fixed 100 + path 200 + words 80 + extras 140 + chip 70 + group 60 + name 100 - 110 over.
        let narrow = title_fit_row(450.0, wants());
        assert_eq!(narrow.path, TitlePathShown::Step { index: 2 }, "the path loses its middle first");
        assert!(!narrow.words, "then the words go");
        assert_eq!(narrow.extras[3], None, "the pull request is the first extra to go");
        let tiny = title_fit_row(250.0, wants());
        assert!(tiny.folded && tiny.chip.is_some_and(|chip| chip <= 70.0));
    }
}
