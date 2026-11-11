//! The title row's looked-up facts: ports and a pull request show once a shell has found them.

use std::path::PathBuf;

use crate::git::pr::{Branch, PrState, PullRequest};
use crate::view::layout::title_parts::TitlePart;
use crate::workspace::Event;

use super::model;

fn part_present(m: &crate::workspace::WorkspaceModel, part: TitlePart) -> bool {
    let title = m.title_snapshot("Macintosh HD");
    title.left.iter().chain(&title.right).any(|slot| slot.part == part && slot.present)
}

#[test]
fn ports_and_a_pull_request_are_present_once_found_and_say_so_with_a_chrome_event() {
    let mut m = model(1);
    m.update_facts(1, |facts| {
        facts.cwd = Some(PathBuf::from("/work/app"));
        facts.repo = Some(PathBuf::from("/work/app"));
        facts.branch = Some("feature/auth".into());
    });
    assert_eq!(m.ports_wanted(), Some(1));
    let branch = Branch { root: PathBuf::from("/work/app"), name: "feature/auth".into() };
    assert_eq!(m.pull_request_branches(), std::slice::from_ref(&branch));
    assert!(!part_present(&m, TitlePart::Ports) && !part_present(&m, TitlePart::PullRequest));

    assert!(m.set_ports(1, vec![3000, 5173]).contains(&Event::Chrome));
    assert!(m.set_ports(1, vec![3000, 5173]).is_empty(), "the same ports are no news");
    let found = PullRequest {
        number: 42,
        url: "https://github.com/o/r/pull/42".into(),
        state: PrState::Open,
        draft: false,
        checks: None,
        review: None,
    };
    assert!(m.set_pull_request(branch, Some(found)).contains(&Event::Chrome));
    assert!(part_present(&m, TitlePart::Ports) && part_present(&m, TitlePart::PullRequest));
    let title = m.title_snapshot("Macintosh HD");
    assert_eq!(title.ports, [3000, 5173]);
    assert_eq!(title.pull_request_words.as_deref(), Some("Pull request #42, open"));

    m.set_ports(1, Vec::new());
    assert!(!part_present(&m, TitlePart::Ports), "a closed listener goes");
}
