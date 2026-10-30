//! The branch picker the Changes tab ("compare with") and the History tab (which branch's log) each
//! have: a [`BranchPicker`] is the state of one (open for a terminal, the branches it listed, its search
//! box), a [`PickerHost`] is the entity that owns one and says what picking a branch does, and a
//! [`PickerChip`] is the chip in a panel card's head that opens it. Each tab has a picker of its own,
//! so neither needs the other to exist.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git;
use wormhole_core::view::panes::Rect;
use wormhole_core::view::picker::{Answer, Cancel, Context as PickerContext, Note, Provider, Query, Row};

use crate::select::{self, Choice, SearchSelect, SelectTrigger};
use crate::settings::UiColors;
use crate::terminal_view::TerminalView;
use crate::window::repo::job::{Job, Ticket};

struct BranchSource {
    head: String,
    branches: RwLock<Vec<String>>,
    current: Option<String>,
}

impl Provider for BranchSource {
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer {
        let needle = select::Needle::new(&query.text);
        let branches = self.branches.read().unwrap_or_else(|error| error.into_inner());
        let mut answer = Answer::default();
        for (key, label) in std::iter::once(("head".to_string(), self.head.as_str()))
            .chain(branches.iter().map(|branch| (branch_key(branch), branch.as_str())))
        {
            if cancel.is_cancelled() {
                return Answer::default();
            }
            if needle.is_empty() || needle.found_in(label) {
                let mut row = Row::new(&key, label);
                row.icon = "arrow.triangle.branch".into();
                let selected =
                    self.current.as_ref().map_or(key == "head", |branch| branch_key(branch) == key);
                row.badge = selected.then(|| "✓".into());
                row.insert = Some(key);
                answer.rows.push(row);
            }
        }
        answer
    }
}

fn branch_key(branch: &str) -> String {
    use std::fmt::Write;
    let mut key = String::from("b");
    for byte in branch.as_bytes() {
        let _ = write!(key, "{byte:02x}");
    }
    key
}

fn branch_from_key(key: &str) -> Option<Option<String>> {
    if key == "head" {
        return Some(None);
    }
    let hex = key.strip_prefix('b')?.as_bytes();
    if hex.is_empty() || hex.len() % 2 != 0 {
        return None;
    }
    let bytes = hex
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| (byte as char).to_digit(16).map(|n| n as u8);
            Some((digit(pair[0])? << 4) | digit(pair[1])?)
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok().map(Some)
}

struct NativeBranchPicker {
    picker: Entity<crate::picker::Picker>,
    source: Arc<BranchSource>,
    _subscription: Subscription,
}

/// Which tab a picker is for: its wording and its ids.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Picker {
    /// Changes tab: compare the working tree with another branch.
    Compare,
    /// History tab: which branch's log to show.
    History,
}

/// The entity that owns a [`BranchPicker`] and decides what picking a branch does.
pub(crate) trait PickerHost: Sized + 'static {
    fn picker(&self) -> &BranchPicker;
    fn picker_mut(&mut self) -> &mut BranchPicker;
    /// The person picked `branch` (`None`: HEAD) for terminal `session`. The picker is closed already.
    fn pick(&mut self, session: i64, branch: Option<String>, cx: &mut Context<Self>);
}

/// One branch picker: whether it is open and for which terminal, the branches it listed, its search box.
pub(crate) struct BranchPicker {
    kind: Picker,
    /// The terminal it is open for. A picker is closed when another terminal is selected.
    open: Option<i64>,
    branches: Vec<String>,
    /// The search box.
    search: Entity<InputState>,
    _subscription: Subscription,
    /// The branch list the open picker waits for.
    job: Job<(i64, PathBuf)>,
    anchor: crate::platform::Anchor,
    native: Option<NativeBranchPicker>,
}

impl BranchPicker {
    /// A closed picker whose search box lives in `host`'s window. Enter in the box picks the first
    /// branch that matches.
    pub(crate) fn new<H: PickerHost>(kind: Picker, window: &mut Window, cx: &mut Context<H>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search branches"));
        let subscription =
            cx.subscribe_in(&search, window, |this: &mut H, _, event: &InputEvent, _, cx| match event {
                InputEvent::Change => cx.notify(),
                InputEvent::PressEnter { .. } => pick_first_match(this, cx),
                _ => {}
            });
        Self {
            kind,
            open: None,
            branches: Vec::new(),
            search,
            _subscription: subscription,
            job: Job::default(),
            anchor: crate::platform::Anchor::default(),
            native: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Closes it and drops what it listed.
    pub(crate) fn close(&mut self) {
        self.open = None;
        self.branches.clear();
        self.native.take();
    }

    /// The branches listed (empty until the list is in).
    #[cfg(test)]
    pub(crate) fn branches(&self) -> &[String] {
        &self.branches
    }

    /// Opens it for `session`, or closes it when it is open. Asks for the repository's branches off the
    /// UI thread. The picker sits in a header that is drawn once git has answered, so a failure to
    /// list them is the panel's to show, not the list's: it just stays empty.
    pub(crate) fn toggle<H: PickerHost>(
        &mut self,
        session: i64,
        terminal: &Entity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<H>,
    ) {
        self.toggle_current(session, terminal, None, window, cx);
    }

    fn toggle_current<H: PickerHost>(
        &mut self,
        session: i64,
        terminal: &Entity<TerminalView>,
        current: Option<String>,
        window: &mut Window,
        cx: &mut Context<H>,
    ) {
        if self.open.is_some() {
            self.close();
            cx.notify();
            return;
        }
        self.open = Some(session);
        self.branches.clear();
        self.search.update(cx, |search, cx| search.set_value("", window, cx));
        self.open_native::<H>(session, terminal, current, window, cx);
        cx.notify();
        let Some(cwd) = terminal.read(cx).cwd.clone() else { return };
        // The same list already being fetched (closed and reopened quickly) is waited for.
        let Some(wanted) = self.job.begin((session, cwd.clone())) else { return };
        let ticket = wanted.ticket();
        let task = cx.background_executor().spawn(async move {
            match git::repo_root(&cwd) {
                Ok(Some(root)) => git::branches(&root).unwrap_or_default(),
                _ => Vec::new(),
            }
        });
        let terminal = terminal.downgrade();
        cx.spawn(async move |this, cx| {
            let branches = task.await;
            let _ = this.update(cx, |this: &mut H, cx| {
                let cwd_now = terminal.read_with(cx, |terminal, _| terminal.cwd.clone()).ok().flatten();
                if this.picker_mut().loaded(ticket, branches, cwd_now.as_deref()) {
                    this.picker_mut().refresh_native(cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn refresh_native(&mut self, cx: &mut App) {
        if let Some(native) = self.native.as_ref() {
            *native.source.branches.write().unwrap_or_else(|error| error.into_inner()) =
                self.branches.clone();
            native.picker.update(cx, |picker, cx| picker.refresh(cx));
        }
    }

    fn open_native<H: PickerHost>(
        &mut self,
        session: i64,
        terminal: &Entity<TerminalView>,
        current: Option<String>,
        window: &mut Window,
        cx: &mut Context<H>,
    ) {
        use crate::picker::{Picker as NativePicker, PickerEvent, Sources, Words};
        let head = match self.kind {
            Picker::Compare => "HEAD (uncommitted changes)",
            Picker::History => "HEAD (current branch)",
        };
        let source = Arc::new(BranchSource {
            head: head.into(),
            branches: RwLock::new(Vec::new()),
            current: current.clone(),
        });
        let anchor = self.anchor.clone();
        let place: crate::picker::Anchor = Rc::new(move |window, _| {
            let viewport = window.viewport_size();
            let bounds = anchor.bounds();
            (
                Rect::new(0.0, 0.0, f32::from(viewport.width), f32::from(viewport.height)),
                Rect::new(
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                ),
            )
        });
        let words = Words {
            placeholder: "Search branches".into(),
            name: "Branches".into(),
            hint: if self.kind == Picker::Compare {
                "Changes since this branch split off from it".into()
            } else {
                String::new()
            },
            note: |note| if matches!(note, Note::Empty) { "No matches".into() } else { String::new() },
        };
        let profile = terminal.read(cx).profile().clone();
        let picker = cx.new(|cx| {
            NativePicker::new(
                PickerContext::default(),
                Sources { providers: vec![source.clone()], decorators: vec![] },
                words,
                profile,
                place,
                window,
                cx,
            )
        });
        if !picker.read(cx).has_panel() {
            picker.update(cx, |picker, cx| picker.close(cx));
            return;
        }
        let preferred = current.as_deref().map_or_else(|| "head".to_string(), branch_key);
        picker.update(cx, |picker, cx| picker.prefer_initial(preferred, cx));
        let subscription = cx.subscribe_in(
            &picker,
            window,
            move |this: &mut H, _, event: &PickerEvent, _, cx| match event {
                PickerEvent::Insert(key) => {
                    let Some(value) = branch_from_key(key) else { return };
                    if this.picker().open != Some(session)
                        || value.as_ref().is_some_and(|branch| !this.picker().branches.contains(branch))
                    {
                        return;
                    }
                    this.picker_mut().close();
                    this.pick(session, value, cx);
                    cx.notify();
                }
                PickerEvent::Close => {
                    this.picker_mut().close();
                    cx.notify();
                }
                _ => {}
            },
        );
        self.native = Some(NativeBranchPicker { picker, source, _subscription: subscription });
    }

    /// The branch list arrived. It is shown only if that same picker is still open for the terminal
    /// and the folder it was asked for; a list that lands after the picker was closed, reopened or
    /// moved to another terminal is dropped.
    fn loaded(&mut self, ticket: Ticket, branches: Vec<String>, cwd_now: Option<&Path>) -> bool {
        let Some(asked) = self.job.finish(ticket) else { return false };
        if branches_wanted(&asked, self.open, cwd_now) {
            self.branches = branches;
            return true;
        }
        false
    }

    /// HEAD (`None`) first, then the repository's branches, each with the branch mark.
    fn options(&self, branch_color: Hsla) -> Vec<Choice<Option<String>>> {
        let head = match self.kind {
            Picker::Compare => "HEAD (uncommitted changes)",
            Picker::History => "HEAD (current branch)",
        };
        std::iter::once(Choice::new(None, head))
            .chain(
                self.branches
                    .iter()
                    .map(|b| Choice::new(Some(b.clone()), b.clone()).mark(IconName::GitBranch, branch_color)),
            )
            .collect()
    }

    /// Type into whichever search field is showing (devctl).
    #[cfg(feature = "devctl")]
    pub(crate) fn set_query(&self, query: &str, window: &mut Window, cx: &mut App) {
        if let Some(native) = self.native.as_ref() {
            native.picker.update(cx, |picker, cx| picker.set_text(query, cx));
        } else {
            self.search.update(cx, |search, cx| search.set_value(query.to_string(), window, cx));
        }
    }

    /// The picker in a panel card's head, as the data its chip draws from: the rows of a list are
    /// drawn from a callback that can't borrow the entity, so the chip is described here and built
    /// there ([`PickerChip::render`]). `prefix` reads muted before the picked branch.
    pub(crate) fn chip<H: PickerHost>(
        &self,
        host: WeakEntity<H>,
        of: ChipOf<'_>,
        branch_color: Hsla,
    ) -> PickerChip<H> {
        let open = self.open.is_some();
        let ChipOf { session, terminal, prefix, branch, current } = of;
        PickerChip {
            kind: self.kind,
            prefix,
            branch: branch.map(str::to_string),
            current,
            open,
            // The menu is built only while it is open; the head re-renders far more often than that.
            options: if open { self.options(branch_color) } else { Vec::new() },
            loading: self.branches.is_empty(),
            search: self.search.clone(),
            session,
            terminal: terminal.clone(),
            this: host,
            anchor: self.anchor.clone(),
            native: self.native.is_some(),
        }
    }
}

/// What a chip shows: the terminal it is for, the words before the branch, the branch it names and the
/// one the picker has as picked (they differ when the picked branch is gone).
pub(crate) struct ChipOf<'a> {
    pub session: i64,
    pub terminal: &'a Entity<TerminalView>,
    pub prefix: &'static str,
    pub branch: Option<&'a str>,
    pub current: Option<String>,
}

/// Enter in the search box: take the first branch that matches.
fn pick_first_match<H: PickerHost>(this: &mut H, cx: &mut Context<H>) {
    let picker = this.picker();
    let Some(session) = picker.open else { return };
    let query = picker.search.read(cx).value().to_string();
    // The colour only marks the rows; matching does not read it.
    let options = picker.options(Hsla::default());
    if let Some(first) = select::filter(&options, &query).into_iter().next() {
        this.picker_mut().close();
        this.pick(session, first.value, cx);
        cx.notify();
    }
}

/// A branch picker's chip and, while it is open, the floating, searchable list of branches under it.
/// It reads `prefix` ("vs", muted) then the picked branch after its mark in `git.branch`, or HEAD.
pub(crate) struct PickerChip<H: PickerHost> {
    kind: Picker,
    prefix: &'static str,
    branch: Option<String>,
    current: Option<String>,
    open: bool,
    options: Vec<Choice<Option<String>>>,
    loading: bool,
    search: Entity<InputState>,
    session: i64,
    terminal: Entity<TerminalView>,
    this: WeakEntity<H>,
    anchor: crate::platform::Anchor,
    native: bool,
}

impl<H: PickerHost> PickerChip<H> {
    pub(crate) fn render(&self, ui: UiColors, cx: &App) -> AnyElement {
        let (kind, prefix, session) = (self.kind, self.prefix, self.session);
        let label = div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .when(!prefix.is_empty(), |d| d.child(div().flex_none().text_color(ui.muted).child(prefix)))
            .when(self.branch.is_some(), |d| {
                d.child(Icon::new(IconName::GitBranch).xsmall().text_color(ui.branch).flex_none())
            })
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(self.branch.clone().unwrap_or_else(|| "HEAD".to_string()))),
            );
        let id = match kind {
            Picker::Compare => "compare-chip",
            Picker::History => "history-chip",
        };
        let (this, on_change) = (self.this.clone(), self.this.clone());
        let terminal = self.terminal.clone();
        let current = self.current.clone();
        let select = SearchSelect::new(
            id,
            ui,
            SelectTrigger::new(SharedString::from(format!("{id}-trigger")), ui, label)
                .compact()
                .native_anchor(self.anchor.clone()),
            &self.search,
            cx,
        )
        .options(self.options.clone())
        .current(self.current.clone())
        .open(self.open && !self.native)
        .anchor(Anchor::TopRight)
        .menu_width(px(320.0))
        .compact()
        .on_open_change(move |want_open, window, cx| {
            on_change
                .update(cx, |this, cx| {
                    if this.picker().is_open() != want_open {
                        this.picker_mut().toggle_current(session, &terminal, current.clone(), window, cx);
                    }
                })
                .ok();
        })
        .on_pick(move |value, _, cx| {
            this.update(cx, |this, cx| {
                this.picker_mut().close();
                this.pick(session, value.clone(), cx);
                cx.notify();
            })
            .ok();
        });
        let select = if self.loading {
            select.hint("Loading branches…")
        } else if kind == Picker::Compare {
            select.hint("Changes since this branch split off from it")
        } else {
            select
        };
        select.into_popover().into_any_element()
    }
}

/// Whether a branch list asked for `asked` (a terminal and its folder) still belongs on screen: the
/// picker is open for that terminal, and the terminal is in that folder still.
fn branches_wanted(asked: &(i64, PathBuf), open: Option<i64>, cwd_now: Option<&Path>) -> bool {
    open == Some(asked.0) && cwd_now == Some(asked.1.as_path())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::RwLock;

    use super::{BranchSource, branch_from_key, branch_key, branches_wanted};
    use crate::window::repo::job::Job;
    use crate::window::repo::job::testing::ManualExecutor;
    use wormhole_core::view::picker::{Cancel, Provider, Query};

    #[test]
    fn native_branch_keys_and_results_preserve_identity_and_current_branch() {
        for branch in ["main", "feature/é", "a b", "HEAD"] {
            assert_eq!(branch_from_key(&branch_key(branch)), Some(Some(branch.to_string())));
        }
        assert_eq!(branch_from_key("head"), Some(None));
        assert_eq!(branch_from_key("b0"), None);
        assert_eq!(branch_from_key("bzz"), None);
        let source = BranchSource {
            head: "HEAD (current branch)".into(),
            branches: RwLock::new(vec!["main".into(), "feature/é".into()]),
            current: Some("main".into()),
        };
        let query =
            |text: &str| source.query(&Query { text: text.into(), ..Default::default() }, &Cancel::new());
        let all = query("");
        assert_eq!(all.rows.len(), 3);
        assert_eq!(all.rows[1].badge.as_deref(), Some("✓"));
        assert_eq!(query("É").rows[0].insert.as_deref(), Some(branch_key("feature/é").as_str()));
        *source.branches.write().unwrap() = vec!["feature/é".into()];
        assert_eq!(query("main").rows.len(), 0);
        let cancelled = Cancel::new();
        cancelled.cancel();
        assert!(source.query(&Query::default(), &cancelled).rows.is_empty());
    }

    const CWD: &str = "/work/repo";

    fn asked(session: i64) -> (i64, PathBuf) {
        (session, PathBuf::from(CWD))
    }

    #[test]
    fn a_list_belongs_to_the_open_picker_of_the_terminal_and_folder_it_was_asked_for() {
        let cwd = Some(Path::new(CWD));
        assert!(branches_wanted(&asked(1), Some(1), cwd));
        // Closed meanwhile, or open for another terminal.
        assert!(!branches_wanted(&asked(1), None, cwd));
        assert!(!branches_wanted(&asked(1), Some(2), cwd));
        // The terminal moved to another folder, or has none.
        assert!(!branches_wanted(&asked(1), Some(1), Some(Path::new("/other"))));
        assert!(!branches_wanted(&asked(1), Some(1), None));
    }

    /// The branches a picker shows, and the job its list goes through.
    struct Picker {
        job: Job<(i64, PathBuf)>,
        shown: Vec<String>,
    }

    /// Opening a picker asks for its branches; the answer goes through the picker's job.
    fn open(exec: &mut ManualExecutor<Picker>, picker: &mut Picker, session: i64, branches: &[&str]) {
        let Some(wanted) = picker.job.begin(asked(session)) else { return };
        let ticket = wanted.ticket();
        let list: Vec<String> = branches.iter().map(|b| b.to_string()).collect();
        exec.spawn(
            move || list,
            move |picker: &mut Picker, list| {
                if picker.job.finish(ticket).is_some() {
                    picker.shown = list;
                }
            },
        );
    }

    #[test]
    fn a_list_for_a_request_that_was_replaced_is_dropped() {
        let mut picker = Picker { job: Job::default(), shown: Vec::new() };
        let mut exec = ManualExecutor::default();
        // Opened for one terminal, then for another before the first list came back.
        open(&mut exec, &mut picker, 1, &["from-one"]);
        open(&mut exec, &mut picker, 2, &["from-two"]);
        // The slower, older answer lands last.
        exec.deliver(1, &mut picker);
        exec.deliver(0, &mut picker);
        assert_eq!(picker.shown, ["from-two"]);
        assert_eq!(exec.pending(), 0);
    }

    #[test]
    fn reopening_the_same_picker_waits_for_the_list_already_on_its_way() {
        let mut picker = Picker { job: Job::default(), shown: Vec::new() };
        let mut exec = ManualExecutor::default();
        open(&mut exec, &mut picker, 1, &["main"]);
        open(&mut exec, &mut picker, 1, &["main"]);
        assert_eq!(exec.pending(), 1, "no second fetch");
        exec.deliver(0, &mut picker);
        assert_eq!(picker.shown, ["main"]);
    }
}
