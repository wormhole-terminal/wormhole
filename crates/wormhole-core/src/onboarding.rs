//! The first-run flow as data: the intro window's welcome page, and the three steps after it.
//! Which step is showing, where Back, Continue, Skip and Start lead, and what the app is asked to
//! do. The three steps are where you work, how it looks, and the coding agents (with the project's
//! trust), or a short primer when no agent is on PATH. A replay of the intro later is the welcome
//! page alone.
//! The first step offers the folders a Finder does ([`places`]) rather than a scan of
//! ~/Projects, and the tips the app shows afterwards can be closed for good ([`DismissedTips`]).
//!
//! Nothing is installed or trusted on the way through: hooks are written only for the agent
//! whose own Install button was pressed, and a folder is trusted only from its "Trust with
//! Touch ID…" button. Navigation never yields either action; the tests drive every path to
//! show it.

use std::path::{Path, PathBuf};

use crate::agents::AgentId;

/// The meta key in the store that says the sheet was finished or skipped.
pub const ONBOARDED_KEY: &str = "onboarded";

/// The meta key in the store that lists the tips closed with "Don't show again".
pub const DISMISSED_TIPS_KEY: &str = "dismissed_tips";

/// A folder the first step offers: where you worked lately, and the places a Finder
/// sidebar starts with. Not everyone has a ~/Projects, so nothing is scanned for repositories.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub kind: PlaceKind,
    /// "Desktop", or a recent folder's name.
    pub name: String,
    pub path: PathBuf,
    /// Where it is, with the home folder as `~`: a recent folder's parent ("~/Projects"), a
    /// standard place's own path ("~/Desktop").
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceKind {
    /// A project folder worked in lately (the menu bar's recent projects).
    Recent,
    Home,
    Desktop,
    Documents,
    /// ~/Developer, the folder Xcode makes; offered only when it exists.
    Developer,
}

/// The folders the first step lists under `home`: the recent project roots that still exist
/// (newest first, as given), then Home, Desktop and Documents, and Developer when there is one.
/// `exists` answers for the disk, so the list is decided without it in tests.
pub fn places(home: &Path, recents: &[PathBuf], exists: impl Fn(&Path) -> bool) -> Vec<Place> {
    let mut places: Vec<Place> = recents
        .iter()
        .filter(|root| exists(root))
        .map(|root| Place {
            kind: PlaceKind::Recent,
            name: folder_name(root),
            path: root.clone(),
            detail: root.parent().map_or_else(|| tilde(root, home), |parent| tilde(parent, home)),
        })
        .collect();
    let standard = [
        (PlaceKind::Home, "Home", home.to_path_buf()),
        (PlaceKind::Desktop, "Desktop", home.join("Desktop")),
        (PlaceKind::Documents, "Documents", home.join("Documents")),
        (PlaceKind::Developer, "Developer", home.join("Developer")),
    ];
    for (kind, name, path) in standard {
        if kind == PlaceKind::Developer && !exists(&path) {
            continue;
        }
        places.push(Place { kind, name: name.to_string(), path: path.clone(), detail: tilde(&path, home) });
    }
    places
}

/// `~/…` for a path in the home folder (`~` for the home folder itself).
pub fn tilde(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

fn folder_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}

/// A tip the app shows while something is new, and that can be closed for good (the sidebar's
/// "No coding agents yet"). Settings → Advanced brings the closed ones back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tip {
    /// The sidebar's note after a first run without agents.
    NoAgents,
}

impl Tip {
    /// The name the store keeps it under.
    fn key(self) -> &'static str {
        match self {
            Tip::NoAgents => "no-agents",
        }
    }
}

/// The tips closed with "Don't show again", as the store keeps them under
/// [`DISMISSED_TIPS_KEY`]: a JSON list of their keys. Keys this build doesn't know are kept,
/// so a newer build's choices survive a trip through an older one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DismissedTips {
    keys: Vec<String>,
}

impl DismissedTips {
    /// What the store holds; anything unreadable counts as nothing dismissed.
    pub fn parse(json: Option<&str>) -> Self {
        let keys = json.and_then(|json| serde_json::from_str::<Vec<String>>(json).ok()).unwrap_or_default();
        Self { keys }
    }

    /// What to put in the store.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.keys).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn contains(&self, tip: Tip) -> bool {
        self.keys.iter().any(|key| key == tip.key())
    }

    /// Close `tip` for good; `false` when it already was.
    pub fn dismiss(&mut self, tip: Tip) -> bool {
        if self.contains(tip) {
            return false;
        }
        self.keys.push(tip.key().to_string());
        true
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Reset tips: every tip shows again.
    pub fn clear(&mut self) {
        self.keys.clear();
    }
}

/// Whether to show the intro window: on a first launch, which nothing was saved from and which never
/// finished or skipped it.
pub fn should_show(onboarded: Option<&str>, saved_windows: usize) -> bool {
    onboarded.is_none() && saved_windows == 0
}

/// A step of the first-run flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The intro window's welcome page, before the three steps.
    Welcome,
    /// "Where do you work?": the folder the first terminal starts in.
    Folder,
    /// "Make it yours": theme, primary, font.
    Look,
    /// "Coding agents": the agents on PATH, each with its own Install, and the folder's trust.
    Agents,
    /// "Ready when you are": no agent on PATH, so a primer instead.
    Primer,
}

impl Step {
    /// The step's place, 1-based, of the three the flow counts; the welcome page is none of them.
    pub fn number(self) -> usize {
        match self {
            Step::Welcome => 0,
            Step::Folder => 1,
            Step::Look => 2,
            Step::Agents | Step::Primer => 3,
        }
    }

    /// The last step: its main button says Start.
    pub fn is_last(self) -> bool {
        matches!(self, Step::Agents | Step::Primer)
    }
}

/// How many steps the flow counts.
pub const STEPS: usize = 3;

/// What the person did on the sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Continue,
    Back,
    /// "Skip" (or Esc): close without changing anything more.
    Skip,
    /// The last step's Start (Get started on a replay's welcome page).
    Start,
    /// A folder was picked on the first step (Open folder…, the home folder).
    PickFolder(PathBuf),
    /// An agent's own Install (or Repair) button.
    Install(AgentId),
    /// The folder's "Trust with Touch ID…" button.
    Trust(PathBuf),
}

/// What the app is to do about an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Nothing beyond showing the step now current.
    None,
    /// Close the intro, remembering it was dealt with; the first terminal stays where it is.
    Skip,
    /// Close the intro, remembering it was dealt with, with the first terminal in `folder` if
    /// one was picked.
    Finish { folder: Option<PathBuf> },
    /// Write the status hooks of exactly this agent.
    InstallHooks(AgentId),
    /// Ask to trust exactly this folder (Touch ID).
    Trust(PathBuf),
}

/// The flow's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Onboarding {
    step: Step,
    /// Whether an agent is on PATH: the third step is the agents' or the primer.
    agents_found: bool,
    folder: Option<PathBuf>,
    /// A replay of the intro: the welcome page is all there is, and it is the last step.
    replay: bool,
}

impl Onboarding {
    /// The first run, from the welcome page.
    pub fn new(agents_found: bool) -> Self {
        Self { step: Step::Welcome, agents_found, folder: None, replay: false }
    }

    /// The intro shown again, once the first run is over: the welcome page alone.
    pub fn replay() -> Self {
        Self { step: Step::Welcome, agents_found: true, folder: None, replay: true }
    }

    /// The flow opened at `step` (devctl, and a restart of the setup from Settings).
    pub fn at(step: Step) -> Self {
        Self { step, agents_found: step != Step::Primer, folder: None, replay: false }
    }

    /// Back to the welcome page (a replay of the intro begins again there), keeping what was picked.
    pub fn to_welcome(&mut self) {
        self.step = Step::Welcome;
    }

    /// Whether the main button says Start (Get started, on a replay's page): the end of the flow.
    pub fn is_last(&self) -> bool {
        self.step.is_last() || (self.replay && self.step == Step::Welcome)
    }

    pub fn step(&self) -> Step {
        self.step
    }

    pub fn folder(&self) -> Option<&PathBuf> {
        self.folder.as_ref()
    }

    /// The agents were looked for (it takes a moment); the third step follows what was found.
    pub fn set_agents_found(&mut self, found: bool) {
        self.agents_found = found;
        if matches!(self.step, Step::Agents | Step::Primer) {
            self.step = self.third();
        }
    }

    fn third(&self) -> Step {
        if self.agents_found { Step::Agents } else { Step::Primer }
    }

    pub fn apply(&mut self, event: Event) -> Action {
        match event {
            Event::Continue if self.is_last() && self.step == Step::Welcome => {
                Action::Finish { folder: None }
            }
            Event::Continue => {
                self.step = match self.step {
                    Step::Welcome => Step::Folder,
                    Step::Folder => Step::Look,
                    Step::Look => self.third(),
                    last => last,
                };
                Action::None
            }
            Event::Back => {
                self.step = match self.step {
                    Step::Welcome | Step::Folder => Step::Welcome,
                    Step::Look => Step::Folder,
                    Step::Agents | Step::Primer => Step::Look,
                };
                Action::None
            }
            Event::Skip => Action::Skip,
            Event::Start if self.is_last() => Action::Finish { folder: self.folder.clone() },
            // Start belongs to the last step; anywhere else it is a Continue.
            Event::Start => self.apply(Event::Continue),
            Event::PickFolder(folder) => {
                self.folder = Some(folder);
                Action::None
            }
            Event::Install(agent) if self.step == Step::Agents => Action::InstallHooks(agent),
            Event::Trust(folder) if self.step == Step::Agents => Action::Trust(folder),
            Event::Install(_) | Event::Trust(_) => Action::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        Action, DismissedTips, Event, Onboarding, PlaceKind, STEPS, Step, Tip, places, should_show, tilde,
    };
    use crate::agents::AgentId;

    /// The first step lists what a Finder would: the recent folders that still exist,
    /// then Home, Desktop and Documents always, and Developer only when it is there.
    #[test]
    fn the_places_are_the_finders_and_the_recent_folders() {
        let home = Path::new("/Users/me");
        let recents = [PathBuf::from("/Users/me/Projects/terminal"), PathBuf::from("/Users/me/gone")];
        let on_disk =
            |path: &Path| path != Path::new("/Users/me/gone") && path != Path::new("/Users/me/Developer");
        let listed = places(home, &recents, on_disk);
        let summary: Vec<(PlaceKind, &str, &str)> =
            listed.iter().map(|p| (p.kind, p.name.as_str(), p.detail.as_str())).collect();
        assert_eq!(
            summary,
            [
                (PlaceKind::Recent, "terminal", "~/Projects"),
                (PlaceKind::Home, "Home", "~"),
                (PlaceKind::Desktop, "Desktop", "~/Desktop"),
                (PlaceKind::Documents, "Documents", "~/Documents"),
            ]
        );
        assert_eq!(listed[2].path, Path::new("/Users/me/Desktop"));
        // With a ~/Developer it is offered last; without recents the list starts at Home.
        let with_developer = places(home, &[], |_| true);
        let kinds: Vec<PlaceKind> = with_developer.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, [PlaceKind::Home, PlaceKind::Desktop, PlaceKind::Documents, PlaceKind::Developer]);
        assert_eq!(tilde(Path::new("/opt/work"), home), "/opt/work", "outside the home folder: as is");
    }

    /// "Don't show again" is kept in the store as a list the next build can add to; Reset
    /// tips empties it.
    #[test]
    fn dismissed_tips_round_trip_and_keep_unknown_keys() {
        let mut tips = DismissedTips::parse(None);
        assert!(tips.is_empty() && !tips.contains(Tip::NoAgents));
        assert!(tips.dismiss(Tip::NoAgents));
        assert!(!tips.dismiss(Tip::NoAgents), "closing it twice changes nothing");
        assert_eq!(tips.to_json(), r#"["no-agents"]"#);
        let again = DismissedTips::parse(Some(r#"["later-tip","no-agents"]"#));
        assert!(again.contains(Tip::NoAgents));
        assert_eq!(again.to_json(), r#"["later-tip","no-agents"]"#, "a newer build's tip survives");
        assert!(DismissedTips::parse(Some("not json")).is_empty(), "unreadable counts as nothing");
        let mut cleared = again;
        cleared.clear();
        assert!(cleared.is_empty() && !cleared.contains(Tip::NoAgents));
    }

    #[test]
    fn it_shows_on_a_first_launch_only() {
        assert!(should_show(None, 0));
        assert!(!should_show(Some("1"), 0), "finished or skipped before");
        assert!(!should_show(None, 2), "windows were saved: not a first launch");
    }

    #[test]
    fn continue_and_back_walk_the_welcome_page_and_the_three_steps() {
        let mut sheet = Onboarding::new(true);
        assert_eq!(sheet.step(), Step::Welcome);
        assert_eq!(sheet.step().number(), 0, "the welcome page is none of the three");
        assert_eq!(sheet.apply(Event::Back), Action::None);
        assert_eq!(sheet.step(), Step::Welcome, "Back stops at the welcome page");
        assert_eq!(sheet.apply(Event::Continue), Action::None);
        assert_eq!(sheet.step(), Step::Folder);
        assert_eq!(sheet.apply(Event::Continue), Action::None);
        assert_eq!(sheet.step(), Step::Look);
        sheet.apply(Event::Continue);
        assert_eq!(sheet.step(), Step::Agents);
        assert_eq!(sheet.step().number(), STEPS);
        sheet.apply(Event::Back);
        assert_eq!(sheet.step(), Step::Look);
        sheet.apply(Event::Back);
        assert_eq!(sheet.step(), Step::Folder);
        sheet.apply(Event::Back);
        assert_eq!(sheet.step(), Step::Welcome, "Back from the first step is the welcome page");
    }

    #[test]
    fn a_replay_of_the_first_run_goes_back_to_the_welcome_page_with_the_folder() {
        let mut sheet = Onboarding::new(true);
        sheet.apply(Event::PickFolder(PathBuf::from("/work/app")));
        sheet.apply(Event::Continue);
        sheet.apply(Event::Continue);
        sheet.to_welcome();
        assert_eq!(sheet.step(), Step::Welcome);
        assert_eq!(sheet.folder(), Some(&PathBuf::from("/work/app")));
    }

    #[test]
    fn a_replay_is_the_welcome_page_alone() {
        let mut replay = Onboarding::replay();
        assert!(replay.is_last(), "its button is the end");
        assert_eq!(replay.apply(Event::Continue), Action::Finish { folder: None });
        assert_eq!(replay.apply(Event::Start), Action::Finish { folder: None });
        assert_eq!(replay.apply(Event::Skip), Action::Skip);
        assert_eq!(replay.step(), Step::Welcome, "it never goes on to the setup");
        // The first run's welcome page is not the end.
        let mut first = Onboarding::new(true);
        assert!(!first.is_last());
        assert_eq!(first.apply(Event::Start), Action::None, "Start before the last step moves on");
        assert_eq!(first.step(), Step::Folder);
    }

    #[test]
    fn without_agents_the_third_step_is_the_primer() {
        let mut sheet = Onboarding::new(false);
        sheet.apply(Event::Continue);
        sheet.apply(Event::Continue);
        sheet.apply(Event::Continue);
        assert_eq!(sheet.step(), Step::Primer);
        assert_eq!(sheet.step().number(), 3);
        // The agents turned up after all (the search finished late).
        sheet.set_agents_found(true);
        assert_eq!(sheet.step(), Step::Agents);
    }

    #[test]
    fn start_finishes_with_the_folder_picked_and_skip_with_nothing() {
        let mut sheet = Onboarding::new(true);
        let folder = PathBuf::from("/work/app");
        sheet.apply(Event::PickFolder(folder.clone()));
        sheet.apply(Event::Continue);
        assert_eq!(sheet.apply(Event::Start), Action::None, "Start before the last step moves on");
        sheet.apply(Event::Continue);
        assert_eq!(sheet.apply(Event::Start), Action::Finish { folder: Some(folder) });
        assert_eq!(Onboarding::new(true).apply(Event::Skip), Action::Skip);
    }

    #[test]
    fn hooks_and_trust_come_only_from_their_own_buttons_on_the_agents_step() {
        let mut sheet = Onboarding::new(true);
        // Not on the welcome, folder or look step, whatever asks.
        assert_eq!(sheet.apply(Event::Install(AgentId::Claude)), Action::None);
        sheet.apply(Event::Continue);
        sheet.apply(Event::Continue);
        sheet.apply(Event::Continue);
        assert_eq!(sheet.apply(Event::Install(AgentId::Codex)), Action::InstallHooks(AgentId::Codex));
        let folder = PathBuf::from("/work/app");
        assert_eq!(sheet.apply(Event::Trust(folder.clone())), Action::Trust(folder));
        // The primer has no agent to install.
        let mut primer = Onboarding::at(Step::Primer);
        assert_eq!(primer.apply(Event::Install(AgentId::Claude)), Action::None);
    }

    /// Every way through the sheet without pressing an agent's Install or a folder's Trust:
    /// nothing is installed or trusted, whatever the order.
    #[test]
    fn onboarding_never_installs_hooks_without_an_explicit_agent() {
        let moves = [
            Event::Continue,
            Event::Back,
            Event::Start,
            Event::Skip,
            Event::PickFolder(PathBuf::from("/work/app")),
        ];
        fn walk(sheet: Onboarding, depth: usize, moves: &[Event], seen: &mut usize) {
            if depth == 0 {
                return;
            }
            for event in moves {
                let mut next = sheet.clone();
                let action = next.apply(event.clone());
                *seen += 1;
                assert!(
                    !matches!(action, Action::InstallHooks(_) | Action::Trust(_)),
                    "{event:?} at {:?} asked for {action:?}",
                    sheet.step()
                );
                if !matches!(action, Action::Skip | Action::Finish { .. }) {
                    walk(next, depth - 1, moves, seen);
                }
            }
        }
        let mut seen = 0;
        for agents in [true, false] {
            walk(Onboarding::new(agents), 6, &moves, &mut seen);
        }
        walk(Onboarding::replay(), 4, &moves, &mut seen);
        assert!(seen > 1000, "the walk covered {seen} moves");
    }
}
