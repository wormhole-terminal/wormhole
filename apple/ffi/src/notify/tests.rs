use super::*;

const SECOND: u64 = 1000;

fn settings() -> NotifySettingsRecord {
    // The bell is off by default; most rules are easier to read with every kind on.
    NotifySettingsRecord { bell: true, ..notify_default_settings() }
}

fn tab(id: i64, state: NoticeStateRecord) -> TabFactsRecord {
    TabFactsRecord {
        session: id,
        tab: id,
        name: format!("terminal {id}"),
        state,
        since_ms: 0,
        who: None,
        source: NoticeSourceRecord::Plain,
        message: None,
        reported: false,
        failure: None,
        notice: None,
        foreground: None,
        folder: None,
        branch: None,
        changed: None,
        selected: false,
        key: false,
        settings: None,
    }
}

fn agent(id: i64, state: NoticeStateRecord) -> TabFactsRecord {
    TabFactsRecord { who: Some("Claude Code".into()), source: NoticeSourceRecord::Agent, ..tab(id, state) }
}

/// One look, away from Wormhole, at `now_ms` on both clocks.
fn look(n: &Notifier, tabs: Vec<TabFactsRecord>, now_ms: u64) -> Vec<NotifyActionRecord> {
    n.observe(tabs, settings(), false, false, now_ms, 1_700_000_000_000 + now_ms)
}

fn posts(actions: &[NotifyActionRecord]) -> Vec<&NoticeRecord> {
    actions
        .iter()
        .filter_map(|a| if let NotifyActionRecord::Post { notice } = a { Some(notice) } else { None })
        .collect()
}

#[test]
fn a_terminal_seen_for_the_first_time_counts_from_idle_and_tells_nothing_by_itself() {
    let n = Notifier::new();
    assert!(look(&n, vec![tab(1, NoticeStateRecord::Idle)], 0).is_empty());
    assert!(look(&n, vec![tab(1, NoticeStateRecord::Idle)], SECOND).is_empty(), "no change, no event");
}

#[test]
fn an_agent_that_needs_you_is_a_time_sensitive_banner_with_its_question() {
    let n = Notifier::new();
    look(&n, vec![agent(7, NoticeStateRecord::Working)], 0);
    let mut asking = agent(7, NoticeStateRecord::NeedsYou);
    asking.message = Some("Approve cargo test?".into());
    asking.since_ms = 5 * SECOND;
    let actions = n.observe(vec![asking], settings(), false, true, 5 * SECOND, 1_700_000_005_000);
    let [notice] = posts(&actions)[..] else { panic!("one banner: {actions:?}") };
    assert_eq!(notice.id, "wormhole/tab/7");
    assert_eq!(notice.thread, "wormhole/terminal/7");
    assert_eq!(notice.title, "terminal 7 needs input");
    assert_eq!(notice.body, "Approve cargo test?");
    assert_eq!(notice.level, NoticeLevelRecord::TimeSensitive);
    assert_eq!(notice.category, NoticeCategoryRecord::NeedsYou);
    assert!(notice.sound);
    // VoiceOver hears it too, at once and urgently.
    assert!(actions.iter().any(
        |a| matches!(a, NotifyActionRecord::Say { text, urgent: true } if text.contains("needs input"))
    ));
    // Without the entitlement it is Active.
    let n = Notifier::new();
    look(&n, vec![agent(7, NoticeStateRecord::Working)], 0);
    let actions = look(&n, vec![agent(7, NoticeStateRecord::NeedsYou)], SECOND);
    assert_eq!(posts(&actions)[0].level, NoticeLevelRecord::Active);
}

#[test]
fn looking_at_the_terminal_or_it_moving_on_takes_the_banner_down() {
    let n = Notifier::new();
    look(&n, vec![agent(1, NoticeStateRecord::Working)], 0);
    look(&n, vec![agent(1, NoticeStateRecord::NeedsYou)], SECOND);
    let moved = look(&n, vec![agent(1, NoticeStateRecord::Working)], 2 * SECOND);
    assert_eq!(moved, [NotifyActionRecord::Retract { id: "wormhole/tab/1".into() }]);
    assert!(look(&n, vec![agent(1, NoticeStateRecord::Working)], 3 * SECOND).is_empty(), "already down");
    // Looking: on screen in the key window while Wormhole is in front.
    look(&n, vec![agent(2, NoticeStateRecord::Working)], 4 * SECOND);
    look(&n, vec![agent(2, NoticeStateRecord::NeedsYou)], 5 * SECOND);
    let mut seen = agent(2, NoticeStateRecord::NeedsYou);
    (seen.selected, seen.key) = (true, true);
    let actions = n.observe(vec![seen], settings(), true, false, 6 * SECOND, 1_700_000_006_000);
    assert_eq!(actions, [NotifyActionRecord::Retract { id: "wormhole/tab/2".into() }]);
}

#[test]
fn the_terminal_in_front_says_nothing_unless_asked() {
    let n = Notifier::new();
    look(&n, vec![agent(1, NoticeStateRecord::Working)], 0);
    let mut asking = agent(1, NoticeStateRecord::NeedsYou);
    asking.selected = false;
    // Wormhole is in front but this terminal is another one: nothing by default.
    let actions = n.observe(vec![asking.clone()], settings(), true, false, SECOND, 1_700_000_001_000);
    assert!(posts(&actions).is_empty(), "{actions:?}");
    // Asked for a banner in front, it comes.
    let n = Notifier::new();
    look(&n, vec![agent(1, NoticeStateRecord::Working)], 0);
    let banner = NotifySettingsRecord { banner_in_front: true, ..settings() };
    let actions = n.observe(vec![asking], banner, true, false, SECOND, 1_700_000_001_000);
    assert_eq!(posts(&actions).len(), 1);
}

#[test]
fn three_finishing_within_ten_seconds_become_one_summary_and_the_singles_go() {
    let n = Notifier::new();
    let working = |ids: &[i64]| ids.iter().map(|&i| agent(i, NoticeStateRecord::Working)).collect::<Vec<_>>();
    look(&n, working(&[1, 2, 3, 4]), 0);
    let done = |ids: &[i64], others: &[i64]| -> Vec<TabFactsRecord> {
        ids.iter()
            .map(|&i| agent(i, NoticeStateRecord::Done))
            .chain(others.iter().map(|&i| agent(i, NoticeStateRecord::Working)))
            .collect()
    };
    assert_eq!(posts(&look(&n, done(&[1], &[2, 3, 4]), 3 * SECOND)).len(), 1);
    assert_eq!(posts(&look(&n, done(&[1, 2], &[3, 4]), 6 * SECOND)).len(), 1);
    let actions = look(&n, done(&[1, 2, 3], &[4]), 8 * SECOND);
    let retracts: Vec<&str> = actions
        .iter()
        .filter_map(|a| if let NotifyActionRecord::Retract { id } = a { Some(id.as_str()) } else { None })
        .collect();
    assert_eq!(retracts, ["wormhole/tab/1", "wormhole/tab/2"], "only banners that were up go");
    let [summary] = posts(&actions)[..] else { panic!("{actions:?}") };
    assert_eq!(summary.id, "wormhole/summary");
    assert_eq!(summary.title, "3 terminals finished");
    assert_eq!(summary.category, NoticeCategoryRecord::Plain);
    assert_eq!(summary.level, NoticeLevelRecord::Active);
}

#[test]
fn a_plain_terminals_bell_is_passive_and_told_once_a_minute() {
    let n = Notifier::new();
    look(&n, vec![tab(1, NoticeStateRecord::Idle)], 0);
    let rang = look(&n, vec![tab(1, NoticeStateRecord::NeedsYou)], SECOND);
    let [bell] = posts(&rang)[..] else { panic!("{rang:?}") };
    assert_eq!(bell.level, NoticeLevelRecord::Passive);
    assert!(!bell.sound, "Notification Center only");
    // It rings again 30 s later: moved on and back, inside the minute: not told.
    look(&n, vec![tab(1, NoticeStateRecord::Idle)], 30 * SECOND);
    assert!(posts(&look(&n, vec![tab(1, NoticeStateRecord::NeedsYou)], 31 * SECOND)).is_empty());
    // A minute after the first it is told again.
    look(&n, vec![tab(1, NoticeStateRecord::Idle)], 40 * SECOND);
    assert_eq!(posts(&look(&n, vec![tab(1, NoticeStateRecord::NeedsYou)], 62 * SECOND)).len(), 1);
}

#[test]
fn a_command_that_ends_is_told_only_past_the_threshold() {
    let wall_start = 1_700_000_000_000u64;
    let n = Notifier::new();
    let mut running = tab(1, NoticeStateRecord::Working);
    running.foreground =
        Some(ForegroundRecord { command: "cargo build".into(), started_unix_ms: wall_start - 38 * SECOND });
    n.observe(vec![running.clone()], settings(), false, false, 0, wall_start);
    running.foreground = None;
    running.state = NoticeStateRecord::Idle;
    let actions = n.observe(vec![running], settings(), false, false, SECOND, wall_start + SECOND);
    let [notice] = posts(&actions)[..] else { panic!("{actions:?}") };
    assert_eq!(notice.title, "cargo build finished", "a plain command is named by what ran");
    assert_eq!(notice.level, NoticeLevelRecord::Passive);
    assert_eq!(n.finished_command(1).map(|f| f.command), Some("cargo build".to_string()));
    // A short one is remembered for its row, not told.
    let mut quick = tab(2, NoticeStateRecord::Working);
    quick.foreground = Some(ForegroundRecord { command: "ls".into(), started_unix_ms: wall_start - SECOND });
    n.observe(vec![quick.clone()], settings(), false, false, 0, wall_start);
    quick.foreground = None;
    assert!(posts(&n.observe(vec![quick], settings(), false, false, SECOND, wall_start)).is_empty());
    assert_eq!(n.finished_command(2).map(|f| f.command), Some("ls".to_string()));
}

#[test]
fn a_failed_command_says_its_exit_code_and_is_not_also_finished() {
    let wall_start = 1_700_000_000_000u64;
    let n = Notifier::new();
    let mut running = tab(1, NoticeStateRecord::Working);
    running.foreground =
        Some(ForegroundRecord { command: "cargo build".into(), started_unix_ms: wall_start - 38 * SECOND });
    n.observe(vec![running.clone()], settings(), false, false, 100 * SECOND, wall_start);
    running.foreground = None;
    running.state = NoticeStateRecord::Failed;
    running.failure = Some(FailureRecord {
        serial: 1,
        code: 101,
        at_ms: 100 * SECOND,
        command: Some("cargo build".into()),
        took_ms: Some(38 * SECOND),
        last_line: Some("error[E0425]".into()),
    });
    let actions = n.observe(vec![running], settings(), false, false, 101 * SECOND, wall_start + SECOND);
    let notices = posts(&actions);
    assert_eq!(notices.len(), 1, "{actions:?}");
    assert!(notices[0].title.contains("failed"), "{}", notices[0].title);
    assert!(notices[0].body.contains("error[E0425]"), "the last line it printed: {}", notices[0].body);
    assert!(notices[0].title.contains("101") || notices[0].subtitle.contains("101"), "{:?}", notices[0]);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, NotifyActionRecord::Say { text, urgent: false } if text.contains("failed")))
    );
}

#[test]
fn a_programs_own_notification_is_told_in_its_words_in_place_of_the_bell() {
    let n = Notifier::new();
    look(&n, vec![tab(1, NoticeStateRecord::Idle)], 0);
    let mut noticed = tab(1, NoticeStateRecord::NeedsYou);
    noticed.notice =
        Some(ProgramNoticeRecord { serial: 1, title: String::new(), body: "Build finished".into() });
    let actions = look(&n, vec![noticed], SECOND);
    let [notice] = posts(&actions)[..] else { panic!("{actions:?}") };
    assert_eq!(notice.body, "Build finished");
    assert_eq!(notice.level, NoticeLevelRecord::Active, "a program meant it");
}

#[test]
fn a_reporting_program_repeating_itself_is_not_told_again_inside_five_seconds() {
    let n = Notifier::new();
    let program = |state, since| TabFactsRecord {
        source: NoticeSourceRecord::Program,
        who: Some("brew".into()),
        reported: true,
        since_ms: since,
        ..tab(1, state)
    };
    look(&n, vec![program(NoticeStateRecord::Working, 0)], 0);
    assert_eq!(posts(&look(&n, vec![program(NoticeStateRecord::NeedsYou, SECOND)], SECOND)).len(), 1);
    look(&n, vec![program(NoticeStateRecord::Working, 2 * SECOND)], 2 * SECOND);
    assert!(posts(&look(&n, vec![program(NoticeStateRecord::NeedsYou, 3 * SECOND)], 3 * SECOND)).is_empty());
    look(&n, vec![program(NoticeStateRecord::Working, 5 * SECOND)], 5 * SECOND);
    assert_eq!(posts(&look(&n, vec![program(NoticeStateRecord::NeedsYou, 7 * SECOND)], 7 * SECOND)).len(), 1);
}

#[test]
fn the_dock_bounces_once_for_needs_you_while_away() {
    let n = Notifier::new();
    let bounce = NotifySettingsRecord { dock_bounce: true, ..settings() };
    look(&n, vec![agent(1, NoticeStateRecord::Working)], 0);
    let actions = n.observe(
        vec![agent(1, NoticeStateRecord::NeedsYou)],
        bounce,
        false,
        false,
        SECOND,
        1_700_000_001_000,
    );
    assert!(actions.contains(&NotifyActionRecord::Bounce));
}

#[test]
fn a_click_says_what_to_do_and_which_banner_to_take_down() {
    let n = Notifier::new();
    look(&n, vec![agent(9, NoticeStateRecord::Working)], 0);
    look(&n, vec![agent(9, NoticeStateRecord::NeedsYou)], SECOND);
    let click = |action: Option<&str>, reply: Option<&str>| ResponseRecord {
        id: "wormhole/tab/9".into(),
        action: action.map(str::to_string),
        reply: reply.map(str::to_string),
    };
    let reply = n.respond(click(Some("reply"), Some("yes")));
    assert_eq!(reply.kind, ResponseKindRecord::Reply { tab: 9, text: "yes".into() });
    assert_eq!(reply.retract.as_deref(), Some("wormhole/tab/9"), "it was up");
    let again = n.respond(click(None, None));
    assert_eq!(again.kind, ResponseKindRecord::Open { tab: 9 });
    assert_eq!(again.retract, None, "already down");
    assert_eq!(n.respond(click(Some("show-changes"), None)).kind, ResponseKindRecord::ShowChanges { tab: 9 });
    assert_eq!(
        n.respond(click(Some("reply"), None)).kind,
        ResponseKindRecord::Nothing,
        "a reply without text"
    );
    let summary = n.respond(ResponseRecord { id: "wormhole/summary".into(), action: None, reply: None });
    assert_eq!(summary.kind, ResponseKindRecord::Activate);
    let foreign = n.respond(ResponseRecord { id: "someone/else".into(), action: None, reply: None });
    assert_eq!(foreign.kind, ResponseKindRecord::Nothing);
}

#[test]
fn the_test_banner_is_one_that_needs_input_would_be() {
    let n = Notifier::new();
    let notice = n.test_notice("api".into(), settings(), true);
    assert_eq!(notice.title, "api needs input");
    assert_eq!(notice.level, NoticeLevelRecord::TimeSensitive);
    assert_eq!(
        n.respond(ResponseRecord { id: notice.id, action: None, reply: None }).kind,
        ResponseKindRecord::Activate
    );
}

#[test]
fn each_profile_may_have_its_own_settings() {
    let n = Notifier::new();
    look(&n, vec![agent(1, NoticeStateRecord::Working), agent(2, NoticeStateRecord::Working)], 0);
    let silent = NotifySettingsRecord { needs_you: false, ..settings() };
    let mut quiet = agent(2, NoticeStateRecord::NeedsYou);
    quiet.settings = Some(silent);
    let actions = look(&n, vec![agent(1, NoticeStateRecord::NeedsYou), quiet], SECOND);
    let ids: Vec<&str> = posts(&actions).iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["wormhole/tab/1"]);
}

#[test]
fn the_badge_counts_by_its_mode() {
    assert_eq!(notify_dock_badge(DockBadgeModeRecord::Off, 3, 2), None);
    assert_eq!(notify_dock_badge(DockBadgeModeRecord::NeedsYou, 2, 5).as_deref(), Some("2"));
    assert_eq!(notify_dock_badge(DockBadgeModeRecord::NeedsYouDone, 2, 1).as_deref(), Some("3"));
    assert_eq!(notify_dock_badge(DockBadgeModeRecord::NeedsYou, 1200, 0).as_deref(), Some("999+"));
    assert_eq!(notify_level(NoticeKindRecord::Bell, true), NoticeLevelRecord::Passive);
}

#[test]
fn a_reason_is_one_short_line_and_the_data_folder_says_which_profile() {
    assert_eq!(
        super::notice_reason("could not start:\n\tno such file".into()),
        "could not start: no such file"
    );
    let words = super::notice_data_folder_detail("Work".into(), "/x/y".into(), "denied".into());
    assert!(words.contains("Work") && words.contains("/x/y") && words.ends_with("denied"), "{words}");
}
