//! The terminal welcome's door and its ops over a real socket: only the welcome terminal with its token, while
//! its welcome runs, gets through; what passes changes what the page offered and nothing else.

use std::sync::Arc;

use serde_json::{Value, json};
use wormhole_core::config::Config;
use wormhole_core::onboarding::ONBOARDED_KEY;
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::storage::store::Store;

use super::support::{Fixture, eventually, fixture};
use crate::hook_server::WelcomePlan;
use crate::{FontFamilyResolver, HookEvent};

struct NoFonts;
impl FontFamilyResolver for NoFonts {
    fn family(&self, _: String) -> Option<String> {
        None
    }
}

/// A terminal whose welcome has begun (its first run, no animation) and the token for it.
fn welcome(f: &Fixture, import_home: Option<String>) -> (i64, String) {
    let tab = f.terminal();
    let token = f
        .server
        .begin_welcome(
            tab,
            WelcomePlan {
                handover: "cursor".into(),
                first_run: true,
                animates: false,
                reduce_motion: false,
                panel_open: true,
                import_home,
            },
            Arc::new(NoFonts),
        )
        .expect("a token");
    assert_eq!(token.len(), 32, "128 bits of hex");
    (tab, token)
}

fn ask(f: &Fixture, tab: i64, token: &str, op: Value) -> Value {
    f.client().control(json!({"action": "welcome", "token": token, "op": op, "from": tab.to_string()}))
}

fn settings(f: &Fixture) -> wormhole_core::config::Settings {
    Config::load_for(&DataRoot::new(&f.dir).default_profile()).settings
}

#[test]
fn the_state_is_what_the_pages_are_made_of() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    let reply = ask(&f, tab, &token, json!({"kind": "state"}));
    assert_eq!(reply["ok"], true, "{reply}");
    let state = &reply["result"];
    assert_eq!(state["handover"], "cursor");
    assert_eq!(state["first_run"], true);
    assert_eq!(state["wait_ms"], 0, "no animation plays: nothing to wait for");
    assert!(state["look"]["themes"].as_array().is_some_and(|themes| themes.len() >= 6), "{state}");
    assert!(state["look"]["primaries"].as_array().is_some_and(|colours| !colours.is_empty()), "{state}");
    assert!(state["notify"].is_object());
    assert!(state["shown"].is_object(), "the window reports how much it shows: {state}");
    eventually("the agent scan", || {
        ask(&f, tab, &token, json!({"kind": "state"}))["result"]["agents"].is_object()
    });
}

#[test]
fn only_that_terminal_with_that_token_gets_through() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    let other = f.terminal();
    let set = json!({"kind": "set_look", "ligatures": false});
    for (from, token) in [(tab, "0".repeat(32)), (other, token.clone()), (tab, String::new())] {
        let reply = ask(&f, from, &token, set.clone());
        assert_eq!(reply["error"]["code"], "refused", "{reply}");
    }
    assert!(settings(&f).font_ligatures, "nothing was changed by a refused request");
    assert_eq!(ask(&f, tab, &token, set)["ok"], true);
    assert!(!settings(&f).font_ligatures);
}

#[test]
fn a_look_change_is_one_the_page_offers_and_lands_in_config_json() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    assert_eq!(ask(&f, tab, &token, json!({"kind": "set_look", "theme": "Nord"}))["ok"], true);
    assert_eq!(settings(&f).theme, "Nord");
    let not_offered = ask(&f, tab, &token, json!({"kind": "set_look", "theme": "No Such Theme"}));
    assert_eq!(not_offered["error"]["code"], "not_found", "{not_offered}");
    assert_eq!(settings(&f).theme, "Nord");
    let size = settings(&f).font_size;
    assert_eq!(ask(&f, tab, &token, json!({"kind": "set_look", "font_step": 1}))["ok"], true);
    assert_eq!(settings(&f).font_size, size + 1.0);
    let state = ask(&f, tab, &token, json!({"kind": "state"}));
    assert_eq!(state["result"]["look"]["theme"], "Nord", "the next state shows it");
}

#[test]
fn notifications_change_only_what_is_named_and_ask_macos_only_from_a_press() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    f.server.set_notification_permission("not_asked".into());
    f.events();
    ask(&f, tab, &token, json!({"kind": "state"}));
    ask(&f, tab, &token, json!({"kind": "set_notify", "sound": "none"}));
    assert!(!f.events().contains(&HookEvent::WelcomeAskPermission), "no press turned a banner on");
    ask(&f, tab, &token, json!({"kind": "set_notify", "needs_you": false}));
    assert!(!settings(&f).notify_needs_you);
    assert!(!f.events().contains(&HookEvent::WelcomeAskPermission), "turning one off asks nothing");
    assert_eq!(ask(&f, tab, &token, json!({"kind": "set_notify", "needs_you": true}))["ok"], true);
    assert!(settings(&f).notify_needs_you);
    assert!(f.events().contains(&HookEvent::WelcomeAskPermission));
}

#[test]
fn the_example_band_is_asked_for_and_never_a_terminal() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    f.events();
    assert_eq!(ask(&f, tab, &token, json!({"kind": "demo", "show": true}))["ok"], true);
    assert!(f.events().contains(&HookEvent::WelcomeDemo { terminal: tab, show: true }));
    let listed = f.client().control(json!({"action": "list"}));
    assert_eq!(listed["result"].to_string().matches("\"terminals\"").count(), 1, "{listed}");
}

#[test]
fn an_import_brings_what_that_terminal_ticks_by_default_from_the_home_it_was_pointed_at() {
    let home = tempfile::tempdir().expect("a scratch home");
    let dir = home.path().join(".config/ghostty");
    std::fs::create_dir_all(&dir).expect("folder");
    crate::settings::tests::write_file(&dir.join("config"), "font-size = 17\nbackground-opacity = 0.9\n");
    let f = fixture("open");
    let (tab, token) = welcome(&f, Some(home.path().to_string_lossy().into_owned()));
    let reply = ask(&f, tab, &token, json!({"kind": "import", "source": "ghostty"}));
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(reply["result"]["brought"].as_u64().is_some_and(|n| n >= 2), "{reply}");
    assert_eq!(settings(&f).font_size, 17.0);
    let missing = ask(&f, tab, &token, json!({"kind": "import", "source": "kitty"}));
    assert_eq!(missing["error"]["code"], "not_found", "{missing}");
}

#[test]
fn finishing_is_remembered_and_the_token_opens_nothing_afterwards() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    f.events();
    assert_eq!(ask(&f, tab, &token, json!({"kind": "finish", "skipped": false}))["ok"], true);
    assert!(f.events().contains(&HookEvent::WelcomeEnded { terminal: tab }));
    let flag = Store::open_at(&f.dir).expect("store").meta(ONBOARDED_KEY).expect("meta");
    assert_eq!(flag.as_deref(), Some("1"), "the first run is remembered");
    for op in [json!({"kind": "state"}), json!({"kind": "set_look", "ligatures": false})] {
        let reply = ask(&f, tab, &token, op);
        assert_eq!(reply["error"]["code"], "refused", "{reply}");
    }
    assert!(settings(&f).font_ligatures);
}

#[test]
fn a_first_run_closed_unfinished_counts_as_skipped_and_the_token_is_gone() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    assert!(f.server.welcome_gone(tab), "it was a first run still running");
    assert_eq!(ask(&f, tab, &token, json!({"kind": "state"}))["error"]["code"], "refused");
    let flag = Store::open_at(&f.dir).expect("store").meta(ONBOARDED_KEY).expect("meta");
    assert_eq!(flag.as_deref(), Some("1"));
}

#[test]
fn how_much_to_show_writes_only_what_differs_and_asks_swift_to_move_the_panel() {
    let f = fixture("open");
    let (tab, token) = welcome(&f, None);
    let state = ask(&f, tab, &token, json!({"kind": "state"}));
    let from = state["result"]["shown"].clone();
    assert_eq!(from["panel_open"], true);
    let mut shown = from.clone();
    shown["panel_open"] = json!(false);
    f.events();
    let reply = ask(&f, tab, &token, json!({"kind": "set_shown", "from": from, "shown": shown}));
    assert_eq!(reply["result"]["panel_open"], false, "{reply}");
    assert!(f.events().contains(&HookEvent::WelcomePanel { terminal: tab, open: false }));
    let state = ask(&f, tab, &token, json!({"kind": "state"}));
    assert_eq!(state["result"]["shown"]["panel_open"], false, "the next state says so");
}
