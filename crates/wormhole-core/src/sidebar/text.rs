//! The words and widths of a row and a band: what a terminal's second
//! line says, how its times read, and which part of a band gives way first when it is narrow.

use std::path::Path;
use std::time::Duration;

use super::filter::RowState;
use super::group::{display_path, place_in_folder};

/// What the second line of a row whose status something tells (an agent, or a program that
/// reports its own) says, or `None` for a one-line row: "Needs you · Approve cargo test?",
/// "Editing sidebar.rs…" (its own words) or "Working · 3m" ("Working · <1m" in the first minute),
/// "Finished · 12m ago", "Failed · rate limit reached". `since` is how long it has been in
/// `state`. A result with words of its own ("Upgraded 12 packages") says them in place of when.
pub fn agent_detail(state: RowState, message: Option<&str>, since: Duration) -> Option<String> {
    let message = message.map(str::trim).filter(|m| !m.is_empty());
    match state {
        RowState::NeedsYou => Some(match message {
            Some(message) => format!("Needs you · {message}"),
            None => "Needs you".to_string(),
        }),
        RowState::Working => Some(match message {
            Some(message) => message.to_string(),
            None => format!("Working · {}", took(since).as_deref().unwrap_or(UNDER_A_MINUTE)),
        }),
        RowState::Finished => {
            Some(format!("Finished · {}", message.map_or_else(|| ago(since), str::to_string)))
        }
        RowState::Failed => Some(format!("Failed · {}", message.map_or_else(|| ago(since), str::to_string))),
        RowState::Exited => Some("Exited".to_string()),
        RowState::Idle => None,
    }
}

/// "1 terminal", "3 terminals".
pub fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Work that has gone on for less than a minute: times count whole minutes, never seconds.
pub const UNDER_A_MINUTE: &str = "<1m";

/// How long something has taken, short: `None` under a minute, then "3m", "1h", "2d".
pub fn took(elapsed: Duration) -> Option<String> {
    let minutes = elapsed.as_secs() / 60;
    match minutes {
        0 => None,
        1..60 => Some(format!("{minutes}m")),
        60..1440 => Some(format!("{}h", minutes / 60)),
        _ => Some(format!("{}d", minutes / 1440)),
    }
}

/// How long ago, short: "just now", "12m ago", "1h ago", "2d ago".
pub fn ago(elapsed: Duration) -> String {
    took(elapsed).map_or_else(|| "just now".to_string(), |took| format!("{took} ago"))
}

/// A running command's time on the clock: "0:05", "1:12", "1:02:03".
pub fn clock(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60);
    if hours > 0 { format!("{hours}:{minutes:02}:{seconds:02}") } else { format!("{minutes}:{seconds:02}") }
}

/// A path for a narrow line: cut from the front, so the end, which tells two apart, stays
/// (`…/clients/acme/app`). Keeps whole components; `max` counts characters.
pub fn path_tail(path: &str, max: usize) -> String {
    if path.chars().count() <= max {
        return path.to_string();
    }
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let mut tail = String::new();
    for part in parts.iter().rev() {
        let next = if tail.is_empty() { part.to_string() } else { format!("{part}/{tail}") };
        // "…/" takes two characters.
        if next.chars().count() + 2 > max && !tail.is_empty() {
            break;
        }
        tail = next;
    }
    format!("…/{tail}")
}

/// What a comfortable row's second line says when the terminal has nothing to report (an idle
/// agent, a bare shell), so no row is a single line: where it is. Under a band that is how far
/// below the band's folder it has gone (`crates/core`), and at the folder itself the branch,
/// since the header already names the folder; in the flat list the path, `~/…` and cut from the
/// front to `max` characters. `None` only when the terminal's folder is unknown.
pub fn place_detail(
    cwd: Option<&Path>,
    band: Option<&Path>,
    branch: Option<&str>,
    home: Option<&Path>,
    max: usize,
) -> Option<String> {
    let cwd = cwd?;
    let Some(folder) = band else {
        return Some(path_tail(&display_path(cwd, home), max));
    };
    match place_in_folder(cwd, folder) {
        Some(deeper) => Some(path_tail(&deeper, max)),
        None => Some(branch.map_or_else(|| path_tail(&display_path(cwd, home), max), str::to_string)),
    }
}

/// How wide a band's folder name and branch chip are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandWidths {
    pub folder: f32,
    pub chip: Option<f32>,
}

/// The order of giving way on a band whose title area (after the chevron, before the
/// count or ±N, which never shrink) is `avail` wide: the folder name keeps at most
/// `folder_max` of it (a share, 0.6); the branch chip shrinks first, from its natural width
/// down to `chip_min` (its glyph and three letters); then the folder name. `gap` is the space
/// between the two. Natural widths are what the text needs.
pub fn band_widths(
    avail: f32,
    folder: f32,
    chip: Option<f32>,
    gap: f32,
    folder_max: f32,
    chip_min: f32,
) -> BandWidths {
    let folder = folder.min(avail * folder_max).max(0.0);
    let Some(chip) = chip else {
        return BandWidths { folder: folder.min(avail.max(0.0)), chip: None };
    };
    let room = (avail - folder - gap).max(0.0);
    let chip_min = chip_min.min(chip);
    let chip = chip.min(room).max(chip_min);
    let folder = folder.min((avail - gap - chip).max(0.0));
    BandWidths { folder, chip: Some(chip) }
}

/// How wide the parts of a folder band's second line are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandLine {
    /// The folder's path; 0 when it is left out.
    pub path: f32,
    pub chip: Option<f32>,
}

/// The order of giving way on a folder band's second line, which has `avail` for the path,
/// the "±N" (`changes` wide, 0 without one: it never gives way) and the branch chip, `gap` apart:
/// the path shortens first (cut with an ellipsis), then the chip down to `chip_min`. A path
/// that would be left narrower than `path_least` is left out instead of showing as a sliver,
/// and the room it and its gap took goes to the chip. Natural widths are what the text needs.
pub fn band_line_widths(
    avail: f32,
    path: f32,
    changes: f32,
    chip: Option<f32>,
    gap: f32,
    path_least: f32,
    chip_min: f32,
) -> BandLine {
    let fit = |with_path: bool| {
        let path = if with_path { path.max(0.0) } else { 0.0 };
        let drawn = usize::from(path > 0.0) + usize::from(changes > 0.0) + usize::from(chip.is_some());
        let room = avail - changes - drawn.saturating_sub(1) as f32 * gap;
        give_way(room, &[Part::down_to(path, 0.0), Part::down_to(chip.unwrap_or(0.0), chip_min)])
    };
    let widths = fit(true);
    let (path, chip_w) = if path > 0.0 && widths[0] < path_least.min(path) {
        (0.0, fit(false)[1])
    } else {
        (widths[0], widths[1])
    };
    BandLine { path, chip: chip.map(|_| chip_w) }
}

/// One part of a line that gives way when the line is too narrow: the width it wants, the least
/// it may be drawn at, and whether it is drawn whole or not at all (words cut to "wo…" say
/// nothing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub natural: f32,
    pub least: f32,
    pub whole: bool,
}

impl Part {
    /// A part that may shrink to `least`.
    pub fn down_to(natural: f32, least: f32) -> Self {
        Self { natural, least: least.min(natural), whole: false }
    }

    /// A part that shows whole or not at all.
    pub fn whole(natural: f32) -> Self {
        Self { natural, least: 0.0, whole: true }
    }
}

/// The widths of `parts` on a line with `avail` for them, the parts giving way in the order
/// given: the first down to its least before the second gives anything, and so on. Whatever
/// still doesn't fit after every part is at its least is the caller's to clip.
pub fn give_way(avail: f32, parts: &[Part]) -> Vec<f32> {
    let mut over = parts.iter().map(|p| p.natural).sum::<f32>() - avail;
    parts
        .iter()
        .map(|part| {
            if over <= 0.0 {
                return part.natural;
            }
            if part.whole {
                over -= part.natural;
                return 0.0;
            }
            let cut = (part.natural - part.least).max(0.0).min(over);
            over -= cut;
            part.natural - cut
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        BandLine, BandWidths, Part, agent_detail, ago, band_line_widths, band_widths, clock, give_way,
        path_tail, place_detail, took,
    };
    use crate::sidebar::RowState;
    use std::path::Path;
    use std::time::Duration;

    const fn mins(n: u64) -> Duration {
        Duration::from_secs(n * 60)
    }

    #[test]
    fn an_agents_second_line_says_what_it_does() {
        let ask = Some("Approve cargo test?");
        assert_eq!(
            agent_detail(RowState::NeedsYou, ask, mins(1)).as_deref(),
            Some("Needs you · Approve cargo test?")
        );
        assert_eq!(agent_detail(RowState::NeedsYou, Some("  "), mins(1)).as_deref(), Some("Needs you"));
        assert_eq!(
            agent_detail(RowState::Working, Some("Editing sidebar.rs…"), mins(3)).as_deref(),
            Some("Editing sidebar.rs…")
        );
        assert_eq!(agent_detail(RowState::Working, None, mins(3)).as_deref(), Some("Working · 3m"));
        // Minutes only: the first minute reads "<1m" and doesn't tick.
        assert_eq!(
            agent_detail(RowState::Working, None, Duration::from_secs(20)).as_deref(),
            Some("Working · <1m")
        );
        assert_eq!(agent_detail(RowState::Finished, None, mins(12)).as_deref(), Some("Finished · 12m ago"));
        assert_eq!(agent_detail(RowState::Finished, None, mins(0)).as_deref(), Some("Finished · just now"));
        assert_eq!(agent_detail(RowState::Idle, ask, mins(5)), None, "an idle agent is one line");
        // A result says its own words when it has them, else when it happened.
        assert_eq!(
            agent_detail(RowState::Finished, Some("Upgraded 12 packages"), mins(2)).as_deref(),
            Some("Finished · Upgraded 12 packages")
        );
        assert_eq!(agent_detail(RowState::Failed, None, mins(2)).as_deref(), Some("Failed · 2m ago"));
        assert_eq!(
            agent_detail(RowState::Failed, Some("rsync failed"), mins(2)).as_deref(),
            Some("Failed · rsync failed")
        );
        assert_eq!(agent_detail(RowState::Failed, Some(" "), mins(0)).as_deref(), Some("Failed · just now"));
    }

    #[test]
    fn times_read_short() {
        assert_eq!(took(Duration::from_secs(59)), None);
        assert_eq!(took(mins(3)).as_deref(), Some("3m"));
        assert_eq!(took(mins(61)).as_deref(), Some("1h"));
        assert_eq!(took(mins(60 * 49)).as_deref(), Some("2d"));
        assert_eq!(ago(mins(60)), "1h ago");
        assert_eq!(clock(Duration::from_secs(5)), "0:05");
        assert_eq!(clock(Duration::from_secs(72)), "1:12");
        assert_eq!(clock(Duration::from_secs(3723)), "1:02:03");
    }

    #[test]
    fn paths_give_way_at_the_front() {
        assert_eq!(path_tail("~/Projects/app", 20), "~/Projects/app");
        assert_eq!(path_tail("~/Projects/clients/acme/app", 18), "…/clients/acme/app");
        assert_eq!(
            path_tail("/a/very-long-folder-name", 6),
            "…/very-long-folder-name",
            "one component stays whole"
        );
    }

    #[test]
    fn a_quiet_row_says_where_it_is() {
        let home = Some(Path::new("/Users/me"));
        let (cwd, folder) = (Path::new("/Users/me/Projects/app"), Path::new("/Users/me/Projects/app"));
        let say = |cwd, band, branch| place_detail(cwd, band, branch, home, 38);
        // The flat list: the path, from home.
        assert_eq!(say(Some(cwd), None, Some("main")).as_deref(), Some("~/Projects/app"));
        assert_eq!(say(Some(Path::new("/opt/x")), None, None).as_deref(), Some("/opt/x"));
        // Under a band: how far below its folder; at the folder, the branch, else the path.
        let deeper = Path::new("/Users/me/Projects/app/crates/core");
        assert_eq!(say(Some(deeper), Some(folder), Some("main")).as_deref(), Some("crates/core"));
        assert_eq!(say(Some(cwd), Some(folder), Some("feature/x")).as_deref(), Some("feature/x"));
        assert_eq!(say(Some(cwd), Some(folder), None).as_deref(), Some("~/Projects/app"));
        // A long path is cut from the front.
        let long = Path::new("/Users/me/Projects/clients/acme/very/deep/app");
        assert_eq!(place_detail(Some(long), None, None, home, 20).as_deref(), Some("…/acme/very/deep/app"));
        // Without a folder there is nothing to say.
        assert_eq!(say(None, None, Some("main")), None);
    }

    #[test]
    fn the_branch_chip_gives_way_first_then_the_folder_name() {
        let widths = |avail, folder, chip| band_widths(avail, folder, chip, 8.0, 0.6, 56.0);
        // Room for everything.
        assert_eq!(widths(300.0, 80.0, Some(70.0)), BandWidths { folder: 80.0, chip: Some(70.0) });
        // The folder name never takes more than 60 %.
        let capped = widths(200.0, 400.0, None);
        assert!((capped.folder - 120.0).abs() < 0.01 && capped.chip.is_none(), "{capped:?}");
        // Narrower: the chip shrinks, the folder keeps its width...
        assert_eq!(widths(200.0, 100.0, Some(112.0)), BandWidths { folder: 100.0, chip: Some(92.0) });
        // ...down to 56, and only then does the folder name give way.
        assert_eq!(widths(150.0, 90.0, Some(112.0)), BandWidths { folder: 86.0, chip: Some(56.0) });
        // A short branch keeps its own width as its least.
        assert_eq!(widths(100.0, 60.0, Some(40.0)), BandWidths { folder: 52.0, chip: Some(40.0) });
        // Nothing left: nothing negative.
        assert_eq!(widths(10.0, 60.0, Some(70.0)), BandWidths { folder: 0.0, chip: Some(56.0) });
    }

    #[test]
    fn a_bands_second_line_shortens_the_path_first_then_the_chip() {
        let line = |avail, path, changes, chip| band_line_widths(avail, path, changes, chip, 8.0, 24.0, 56.0);
        // Room for everything: 100 + 8 + 20 + 8 + 80.
        assert_eq!(line(300.0, 100.0, 20.0, Some(80.0)), BandLine { path: 100.0, chip: Some(80.0) });
        // The path gives way alone, the "±N" and the chip keep theirs.
        assert_eq!(line(200.0, 100.0, 20.0, Some(80.0)), BandLine { path: 84.0, chip: Some(80.0) });
        // Without git there is only the path, which takes what there is.
        assert_eq!(line(60.0, 100.0, 0.0, None), BandLine { path: 60.0, chip: None });
        // A path left under the least is left out; its gap goes to the chip, down to its least.
        assert_eq!(line(120.0, 100.0, 20.0, Some(80.0)), BandLine { path: 0.0, chip: Some(80.0) });
        assert_eq!(line(70.0, 100.0, 20.0, Some(112.0)), BandLine { path: 0.0, chip: Some(56.0) });
        assert_eq!(line(10.0, 100.0, 20.0, Some(112.0)), BandLine { path: 0.0, chip: Some(56.0) });
        // A short chip keeps its own width as its least, and the "±N" alone is never cut.
        assert_eq!(line(40.0, 100.0, 20.0, Some(30.0)), BandLine { path: 0.0, chip: Some(30.0) });
        assert_eq!(line(10.0, 100.0, 20.0, None), BandLine { path: 0.0, chip: None });
    }

    #[test]
    fn parts_give_way_one_after_the_other() {
        let parts = [Part::down_to(50.0, 10.0), Part::whole(30.0), Part::down_to(40.0, 0.0)];
        assert_eq!(give_way(200.0, &parts), [50.0, 30.0, 40.0], "room for all");
        assert_eq!(give_way(100.0, &parts), [30.0, 30.0, 40.0], "the first gives way alone");
        // The first at its least, the whole part goes entirely, and what that frees stays free.
        assert_eq!(give_way(70.0, &parts), [10.0, 0.0, 40.0]);
        assert_eq!(give_way(30.0, &parts), [10.0, 0.0, 20.0]);
        // Too narrow even at the least: the rest is clipped, nothing is negative.
        assert_eq!(give_way(0.0, &parts), [10.0, 0.0, 0.0]);
        // A least above the natural width means the part never shrinks.
        assert_eq!(Part::down_to(20.0, 56.0).least, 20.0);
    }
}
