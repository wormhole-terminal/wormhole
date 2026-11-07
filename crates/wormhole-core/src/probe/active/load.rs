//! Reading the Probes that are on.

use crate::probe::approval::{self, Approval, ApprovalStore, ProbeState};
use crate::probe::hooks::CompiledHooks;
use crate::probe::manifest::ActionKind;
use crate::probe::read::ProbeFolder;
use crate::probe::rules::CompiledRules;
use crate::probe::validate::{DeclaredPermission, https_host, validate};
use crate::storage::paths::DataRoot;

/// What the user granted of what a Probe declared, as the runtime asks it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grants {
    /// Hosts a rule's link may open.
    pub open: Vec<String>,
    pub notify: bool,
    /// Text the terminals hold may reach a hook.
    pub terminal_read: bool,
}

impl Grants {
    fn of(approval: &Approval, declared: &[DeclaredPermission]) -> Self {
        let granted = |key: &'static str| declared.iter().filter(move |p| p.key == key && approval.grants(p));
        Self {
            open: granted("open").map(|p| p.detail.clone()).collect(),
            notify: granted("notify").next().is_some(),
            terminal_read: granted("terminal").any(|p| p.detail.starts_with("read")),
        }
    }
}

/// One Probe that is on.
#[derive(Debug, Clone)]
pub struct ActiveProbe {
    pub id: String,
    pub name: String,
    /// Its rules, with the actions the user did not grant, and the ones this build does not run, taken off.
    pub rules: CompiledRules,
    pub hooks: CompiledHooks,
    pub grants: Grants,
    /// The ids of its `[[contributes.status]]` items: the only ones a hook may set.
    pub status_items: Vec<String>,
}

/// The Probes that are on and why the others are not.
#[derive(Debug, Default)]
pub struct Loaded {
    pub probes: Vec<ActiveProbe>,
    /// A Probe that is on but could not run, with the reason, for the log.
    pub skipped: Vec<(String, String)>,
}

/// Every Probe of `root` that is on as it stands now: approved and sealed, the folder as approved, the copy it
/// runs from re-hashed (`approval::runnable`). Reads each Probe's files and the key (which may prompt), so call
/// it off the main thread.
pub fn load(root: &DataRoot) -> Loaded {
    let mut loaded = Loaded::default();
    let Ok(listing) = approval::list(root) else { return loaded };
    let store = ApprovalStore::load(root);
    for entry in listing.entries.iter().filter(|entry| entry.state == ProbeState::On) {
        let Some(listed) = &entry.probe else { continue };
        let id = listed.id.clone();
        match load_one(root, &store, &id) {
            Ok(probe) => loaded.probes.push(probe),
            Err(why) => loaded.skipped.push((id, why)),
        }
    }
    loaded
}

fn load_one(root: &DataRoot, store: &ApprovalStore, id: &str) -> Result<ActiveProbe, String> {
    // The copy, never the folder the user may be editing: that is what was approved.
    let copy = approval::runnable(root, id).map_err(|err| err.to_string())?;
    let folder = ProbeFolder::read(copy.dir()).map_err(|err| err.to_string())?;
    let approval = store.get(id).ok_or("no approval")?;
    let grants = Grants::of(approval, &validate(&folder).summary.permissions);
    let manifest = &folder.manifest;
    let mut rules = CompiledRules::compile(manifest).map_err(|err| err.to_string())?;
    let hooks = CompiledHooks::compile(manifest).map_err(|err| err.to_string())?;
    // Open a file: nothing to grant, the user's click is the consent. Open a link: only to a granted host.
    // Copy and command are not run by this build.
    rules.restrict_actions(|_, action| match action.kind {
        ActionKind::OpenPath => true,
        ActionKind::OpenUrl => action
            .url
            .as_deref()
            .and_then(|url| https_host(url).ok())
            .is_some_and(|host| grants.open.iter().any(|granted| granted == host)),
        ActionKind::Copy | ActionKind::Command | ActionKind::None => false,
    });
    Ok(ActiveProbe {
        id: manifest.probe.id.clone(),
        name: manifest.probe.name.clone(),
        rules,
        hooks,
        grants,
        status_items: manifest.contributes.status.iter().map(|item| item.id.clone()).collect(),
    })
}

/// The rules of all the Probes as one set, in their order.
pub fn rules_of(probes: &[ActiveProbe]) -> CompiledRules {
    CompiledRules::combine(probes.iter().map(|probe| (probe.id.clone(), probe.rules.clone())))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::probe::approval::{self, Proof};
    use crate::probe::rules::{matches, target};

    const MANIFEST: &str = r#"[probe]
id = "acme-tools"
name = "Acme tools"
description = "d"
author = "a"
version = "0.1.0"
wormhole = ">=1.3"
api = 1

[permissions]
open = ["acme.atlassian.net"]
notify = true

[[contributes.rules]]
id = "ticket"
pattern = '(?-u:\b)(ACME-[0-9]{1,6})(?-u:\b)'
tone = "link"
action = { type = "open-url", url = "https://acme.atlassian.net/browse/{1}" }

[[contributes.rules]]
id = "path"
pattern = '[a-z/]+\.rs:[0-9]+'
tone = "muted"
action = { type = "open-path" }

[[contributes.rules]]
id = "copy"
pattern = 'COPYME'
action = { type = "copy" }

[[contributes.hooks]]
id = "bell"
on = "bell"
do = [{ type = "notify", title = "Bell" }]
"#;

    fn install(root: &DataRoot) {
        let dir = root.probes_dir().join("acme-tools");
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("probe.toml"), MANIFEST).expect("manifest");
    }

    fn approve(root: &DataRoot, granted: impl Fn(&DeclaredPermission) -> bool) {
        let request = approval::read_one(root, "acme-tools").expect("a request");
        let mut grants = request.initial_grants();
        for permission in request.summary().permissions.iter().filter(|p| !granted(p)) {
            grants.set(permission, false);
        }
        let confirmed = request.confirm(&grants, Proof::TouchId).expect("a yes");
        approval::approve(root, &request, &confirmed, 1_800_000_000).expect("approves");
    }

    #[test]
    fn only_a_probe_that_is_on_loads_and_with_what_was_granted() {
        let tmp = TempDir::new().expect("tempdir");
        let root = DataRoot::new(tmp.path().join("data"));
        fs::create_dir_all(root.base()).expect("data dir");
        install(&root);
        assert!(load(&root).probes.is_empty(), "off until approved");

        approve(&root, |_| true);
        let loaded = load(&root);
        assert!(loaded.skipped.is_empty(), "{:?}", loaded.skipped);
        let [probe] = &loaded.probes[..] else { panic!("one Probe: {:?}", loaded.probes) };
        assert_eq!(
            probe.grants,
            Grants { open: vec!["acme.atlassian.net".into()], notify: true, terminal_read: false }
        );
        let rules = rules_of(&loaded.probes);
        let line = "see ACME-42 and src/a.rs:7 or COPYME";
        let found = matches(&rules, line);
        let targets: Vec<_> =
            found.iter().map(|m| target(&rules, m, line).map(|t| t.text().into_owned())).collect();
        assert_eq!(
            targets,
            [Some("https://acme.atlassian.net/browse/ACME-42".into()), Some("src/a.rs:7".into()), None],
            "copy is not run by this build"
        );
        assert_eq!(rules.rule(0).map(|r| r.owner.as_str()), Some("acme-tools"));

        // Without the host, the ticket still has its tone but no link.
        approval::turn_off(&root, "acme-tools").expect("off");
        approve(&root, |p| p.key != "open");
        let rules = rules_of(&load(&root).probes);
        let found = matches(&rules, "ACME-42");
        assert_eq!(found.len(), 1);
        assert_eq!(target(&rules, &found[0], "ACME-42"), None);
        approval::turn_off(&root, "acme-tools").expect("off");
    }

    #[test]
    fn a_probe_changed_since_it_was_approved_does_not_load() {
        let tmp = TempDir::new().expect("tempdir");
        let root = DataRoot::new(tmp.path().join("data"));
        fs::create_dir_all(root.base()).expect("data dir");
        install(&root);
        approve(&root, |_| true);
        let edited = MANIFEST.replace("COPYME", "COPYYOU");
        fs::write(root.probes_dir().join("acme-tools/probe.toml"), edited).expect("edit");
        assert!(load(&root).probes.is_empty());
        approval::turn_off(&root, "acme-tools").expect("off");
    }
}
