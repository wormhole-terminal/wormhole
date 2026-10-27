//! The seal on an approval: what a program that can write the data folder, but cannot read the Keychain,
//! does not get (`probes.md` 6.1). The keys here are stand-ins put in front of the system's (`with_keys`);
//! nothing reaches the Keychain, and the files that do exist are in a scratch folder.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use super::key::tests_keys::with_keys;
use super::key::{KeySource, PURPOSE};
use super::seal::SealKey;
use super::tests::{ACME, NOW, approve_all, install, setup, state};
use super::*;
use crate::storage::paths::DataRoot;
use crate::storage::secret_key::{Key, is_random_id, new_key};

/// A key of `0`'s byte, for any account (or only `only`).
struct Fixed {
    byte: u8,
    only: Option<String>,
}

fn fixed(byte: u8) -> Fixed {
    Fixed { byte, only: None }
}

impl KeySource for Fixed {
    fn find(&self, _: &Path, account: &str) -> Result<Option<Key>> {
        Ok(self.only.as_deref().is_none_or(|only| only == account).then(|| Key::new([self.byte; 32])))
    }
    fn open(&self, _: &Path, _: &str) -> Result<Key> {
        Ok(Key::new([self.byte; 32]))
    }
    fn forget(&self, _: &Path, _: &str) -> Result<()> {
        Ok(())
    }
}

/// A Keychain in memory: a random key per account, kept until it is forgotten. Clones share it.
#[derive(Clone, Default)]
struct Vault(Arc<Mutex<HashMap<String, Key>>>);

impl Vault {
    fn has(&self, account: &str) -> bool {
        self.0.lock().expect("vault").contains_key(account)
    }
}

impl KeySource for Vault {
    fn find(&self, _: &Path, account: &str) -> Result<Option<Key>> {
        Ok(self.0.lock().expect("vault").get(account).cloned())
    }
    fn open(&self, _: &Path, account: &str) -> Result<Key> {
        let key = new_key()?;
        self.0.lock().expect("vault").insert(account.to_string(), key.clone());
        Ok(key)
    }
    fn forget(&self, _: &Path, account: &str) -> Result<()> {
        self.0.lock().expect("vault").remove(account);
        Ok(())
    }
}

/// The Keychain says no: denied, cancelled or locked.
struct Denied;

impl KeySource for Denied {
    fn find(&self, _: &Path, _: &str) -> Result<Option<Key>> {
        Err(anyhow!("denied"))
    }
    fn open(&self, _: &Path, _: &str) -> Result<Key> {
        Err(anyhow!("denied"))
    }
    fn forget(&self, _: &Path, _: &str) -> Result<()> {
        Err(anyhow!("denied"))
    }
}

/// The key is never to be read or made; deleting one is fine.
struct Forbidden;

impl KeySource for Forbidden {
    fn find(&self, _: &Path, _: &str) -> Result<Option<Key>> {
        panic!("the key was read");
    }
    fn open(&self, _: &Path, _: &str) -> Result<Key> {
        panic!("the key was read");
    }
    fn forget(&self, _: &Path, _: &str) -> Result<()> {
        Ok(())
    }
}

fn file_of(root: &DataRoot) -> Value {
    serde_json::from_str(&fs::read_to_string(root.probe_approvals_file()).expect("read")).expect("json")
}

fn write_file(root: &DataRoot, file: &Value) {
    fs::write(root.probe_approvals_file(), serde_json::to_string_pretty(file).expect("json")).expect("write");
}

fn listed(root: &DataRoot, id: &str) -> Entry {
    list(root).expect("lists").entries.into_iter().find(|e| e.folder == id).expect("listed")
}

const ACCOUNT: &str = "0123456789abcdef0123456789abcdef";
const OTHER_ACCOUNT: &str = "fedcba9876543210fedcba9876543210";

fn sample() -> Approval {
    Approval {
        fingerprint: "ab".repeat(32),
        version: "0.3.0".into(),
        declared: vec!["process:git".into(), "notify:".into()],
        granted: vec!["notify:".into()],
        approved_at: NOW,
    }
}

#[test]
fn every_sealed_field_is_in_the_seal() {
    let key = SealKey::new(&Key::new([7; 32]));
    let approval = sample();
    let seal = key.seal(3, "acme-tools", ACCOUNT, &approval);
    assert_eq!(seal.len(), 64, "HMAC-SHA256, in hex");
    assert!(key.verify(3, "acme-tools", ACCOUNT, &approval, &seal));

    let changed: Vec<(&str, Approval)> = vec![
        ("fingerprint", Approval { fingerprint: "cd".repeat(32), ..sample() }),
        ("manifest version", Approval { version: "0.3.1".into(), ..sample() }),
        ("a permission declared", Approval { declared: vec!["notify:".into()], ..sample() }),
        (
            "a permission granted",
            Approval { granted: vec!["notify:".into(), "process:git".into()], ..sample() },
        ),
        ("nothing granted", Approval { granted: vec![], ..sample() }),
        ("the time", Approval { approved_at: NOW + 1, ..sample() }),
    ];
    for (what, approval) in &changed {
        assert!(!key.verify(3, "acme-tools", ACCOUNT, approval, &seal), "{what} is sealed");
    }
    assert!(!key.verify(3, "acme-other", ACCOUNT, &approval, &seal), "the id is sealed");
    assert!(!key.verify(4, "acme-tools", ACCOUNT, &approval, &seal), "the store's version is sealed");
    assert!(!key.verify(3, "acme-tools", OTHER_ACCOUNT, &approval, &seal), "the account is sealed");
    assert!(
        !SealKey::new(&Key::new([8; 32])).verify(3, "acme-tools", ACCOUNT, &approval, &seal),
        "the key is"
    );

    // The lists are sets: the order they were written in is not part of what was approved.
    let reordered = Approval { declared: vec!["notify:".into(), "process:git".into()], ..sample() };
    assert!(key.verify(3, "acme-tools", ACCOUNT, &reordered, &seal));
    // And a seal that is not a seal is refused, not a panic.
    for bad in ["", "zz", &seal[1..], &seal[..62], &seal.to_uppercase().replace('A', "G"), "+f"] {
        assert!(!key.verify(3, "acme-tools", ACCOUNT, &approval, bad), "{bad}");
    }
    // Fields are framed, not joined: moving a boundary between two of them changes the seal.
    let a = Approval { fingerprint: "ab".into(), version: "c".into(), ..sample() };
    let b = Approval { fingerprint: "a".into(), version: "bc".into(), ..sample() };
    assert_ne!(key.seal(3, "x", ACCOUNT, &a), key.seal(3, "x", ACCOUNT, &b));
}

#[test]
fn an_approval_is_sealed_and_reloads_as_on() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    with_keys(fixed(1), || {
        approve_all(&root, "acme-tools");
        assert_eq!(state(&root, "acme-tools"), ProbeState::On);
        assert!(!listed(&root, "acme-tools").unsealed);
        runnable(&root, "acme-tools").expect("runs");
        let file = file_of(&root);
        assert_eq!(file["version"], 3);
        assert!(is_random_id(file["probes"]["acme-tools"]["account"].as_str().expect("an account")));
        assert_eq!(file["probes"]["acme-tools"]["seal"].as_str().map(str::len), Some(64));
    });
}

#[test]
fn an_entry_without_a_seal_or_with_a_wrong_one_is_off_and_never_runnable() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    with_keys(fixed(1), || {
        approve_all(&root, "acme-tools");
        let sealed = file_of(&root);
        // The fingerprint is right, the copy is there, the account is right: all a writer can know.
        for seal in [None, Some(""), Some("not a seal"), Some(&"00".repeat(32)[..])] {
            let mut forged = sealed.clone();
            match seal {
                None => {
                    forged["probes"]["acme-tools"].as_object_mut().expect("entry").remove("seal");
                }
                Some(seal) => forged["probes"]["acme-tools"]["seal"] = json!(seal),
            }
            write_file(&root, &forged);
            assert_eq!(state(&root, "acme-tools"), ProbeState::Off, "{seal:?}");
            let row = listed(&root, "acme-tools");
            assert!(row.unsealed && row.approved && row.can_forget(), "{seal:?}");
            assert!(ApprovalStore::load(&root).get("acme-tools").is_none());
            assert_eq!(runnable(&root, "acme-tools").err(), Some(NotRunnable::Unsealed), "{seal:?}");
        }
        write_file(&root, &sealed);
        assert_eq!(state(&root, "acme-tools"), ProbeState::On, "the real seal still works");
    });
}

#[test]
fn a_changed_field_in_the_file_unseals_the_entry() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    with_keys(fixed(1), || {
        approve_all(&root, "acme-tools");
        let sealed = file_of(&root);
        let edits: [(&str, Value); 5] = [
            ("granted", json!(["process:git", "open:evil.example"])),
            ("declared", json!([])),
            ("approved_at", json!(1)),
            ("version", json!("9.9.9")),
            ("fingerprint", json!("0".repeat(64))),
        ];
        for (field, value) in edits {
            let mut forged = sealed.clone();
            forged["probes"]["acme-tools"][field] = value;
            write_file(&root, &forged);
            assert!(listed(&root, "acme-tools").unsealed, "{field}");
            assert_eq!(state(&root, "acme-tools"), ProbeState::Off, "{field}");
        }
        // The entry moved to another id carries a seal for the first: not valid there.
        let mut moved = sealed.clone();
        let entry = moved["probes"].as_object_mut().expect("probes").remove("acme-tools").expect("entry");
        moved["probes"]["other-one"] = entry;
        write_file(&root, &moved);
        assert!(ApprovalStore::load(&root).get("other-one").is_none());
    });
}

#[test]
fn a_hand_written_approval_with_no_account_is_off_and_asks_for_no_key() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let fingerprint = read_one(&root, "acme-tools").expect("request").fingerprint().to_string();
    let entry = json!({"fingerprint": fingerprint, "version": "0.3.0", "declared": [], "granted": [], "approved_at": 1});
    write_file(&root, &json!({"version": 3, "probes": {"acme-tools": entry}}));
    with_keys(Forbidden, || {
        assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
        assert!(listed(&root, "acme-tools").unsealed);
        assert_eq!(runnable(&root, "acme-tools").err(), Some(NotRunnable::Unsealed));
    });
    // Without the override, the system's dev key is not even made for a read.
    assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
    assert!(!root.base().join(PURPOSE.dev_key_file).exists());
}

#[test]
fn approvals_of_the_last_format_read_as_nothing_approved() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    with_keys(fixed(1), || approve_all(&root, "acme-tools"));
    let mut old = file_of(&root);
    old["version"] = json!(2);
    write_file(&root, &old);
    with_keys(Forbidden, || {
        assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
        assert!(ApprovalStore::load(&root).is_damaged());
    });
}

#[test]
fn another_key_turns_every_entry_off_until_it_is_approved_again() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    install(&root, "quiet-one", ACME);
    with_keys(fixed(1), || {
        approve_all(&root, "acme-tools");
        approve_all(&root, "quiet-one");
    });
    with_keys(fixed(2), || {
        for id in ["acme-tools", "quiet-one"] {
            assert_eq!(state(&root, id), ProbeState::Off, "{id}");
            assert!(listed(&root, id).unsealed, "{id}");
            assert_eq!(runnable(&root, id).err(), Some(NotRunnable::Unsealed), "{id}");
        }
        // Approving again seals this one with the new key and leaves the other as it was.
        approve_all(&root, "acme-tools");
        assert_eq!(state(&root, "acme-tools"), ProbeState::On);
        assert_eq!(state(&root, "quiet-one"), ProbeState::Off);
    });
    with_keys(fixed(1), || assert_eq!(state(&root, "quiet-one"), ProbeState::On));
}

#[test]
fn a_key_that_is_gone_reads_as_unsealed_and_an_entry_that_names_no_key_asks_for_none() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let vault = Vault::default();
    with_keys(vault.clone(), || approve_all(&root, "acme-tools"));
    let sealed = file_of(&root);
    let account = sealed["probes"]["acme-tools"]["account"].as_str().expect("account").to_string();
    with_keys(vault.clone(), || {
        assert_eq!(state(&root, "acme-tools"), ProbeState::On);
        // Pointed at an account whose key is not there: nothing to check the seal with.
        let mut elsewhere = sealed.clone();
        elsewhere["probes"]["acme-tools"]["account"] = json!(OTHER_ACCOUNT);
        write_file(&root, &elsewhere);
        assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
        assert!(listed(&root, "acme-tools").unsealed);
    });
    // Not an account at all: no key is asked for.
    let mut forged = sealed.clone();
    forged["probes"]["acme-tools"]["account"] = json!("../../etc");
    write_file(&root, &forged);
    with_keys(Forbidden, || assert_eq!(state(&root, "acme-tools"), ProbeState::Off));
    // Deleted from the Keychain: the next read counts it, nothing remembers the key.
    write_file(&root, &sealed);
    with_keys(vault.clone(), || {
        assert_eq!(state(&root, "acme-tools"), ProbeState::On);
        vault.forget(Path::new(""), &account).expect("deleted");
        assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
    });
}

/// Everything a writer of the data folder could put back: the approvals file and the approved copies.
fn saved(root: &DataRoot) -> (String, Vec<(std::path::PathBuf, Vec<u8>)>) {
    let file = fs::read_to_string(root.probe_approvals_file()).expect("read");
    let mut copies = Vec::new();
    let mut stack = vec![root.probes_approved_dir()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                copies.push((path.clone(), fs::read(&path).expect("read")));
            }
        }
    }
    (file, copies)
}

fn put_back(root: &DataRoot, (file, copies): &(String, Vec<(std::path::PathBuf, Vec<u8>)>)) {
    fs::write(root.probe_approvals_file(), file).expect("write");
    // A copy still there is read-only and holds the same bytes (the Probe did not change).
    for (path, bytes) in copies.iter().filter(|(path, _)| !path.exists()) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, bytes).expect("write");
    }
}

#[test]
fn a_withdrawn_approval_copied_back_stays_off() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    install(&root, "quiet-one", ACME);
    let vault = Vault::default();
    with_keys(vault.clone(), || {
        approve_all(&root, "acme-tools");
        approve_all(&root, "quiet-one");
        let before = saved(&root);
        let account =
            file_of(&root)["probes"]["acme-tools"]["account"].as_str().expect("account").to_string();
        turn_off(&root, "acme-tools").expect("off");
        assert!(!vault.has(&account), "its key is deleted");
        assert!(file_of(&root).get("retired").is_none(), "and leaves the list once it is");

        put_back(&root, &before);
        assert_eq!(state(&root, "acme-tools"), ProbeState::Off);
        assert!(listed(&root, "acme-tools").unsealed);
        assert_eq!(runnable(&root, "acme-tools").err(), Some(NotRunnable::Unsealed));
        assert_eq!(state(&root, "quiet-one"), ProbeState::On, "the other keeps its own key");
    });
}

#[test]
fn a_replaced_approval_copied_back_stays_off() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let vault = Vault::default();
    with_keys(vault.clone(), || {
        approve_all(&root, "acme-tools");
        let wide = saved(&root);
        let first = file_of(&root)["probes"]["acme-tools"]["account"].as_str().expect("account").to_string();
        // Approved again with nothing granted: narrower, under a new key.
        let request = read_one(&root, "acme-tools").expect("request");
        let mut grants = request.initial_grants();
        for permission in &request.summary().permissions {
            grants.set(permission, false);
        }
        let confirmed = request.confirm(&grants, Proof::TouchId).expect("a yes");
        approve(&root, &request, &confirmed, NOW + 1).expect("approved");
        let second = file_of(&root)["probes"]["acme-tools"]["account"].as_str().expect("account").to_string();
        assert_ne!(first, second);
        assert!(!vault.has(&first) && vault.has(&second));

        put_back(&root, &wide);
        assert_eq!(state(&root, "acme-tools"), ProbeState::Off, "the wider grants don't come back");
    });
}

#[test]
fn a_key_a_write_could_not_delete_is_deleted_by_the_next_one() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let vault = Vault::default();
    with_keys(vault.clone(), || approve_all(&root, "acme-tools"));
    let account = file_of(&root)["probes"]["acme-tools"]["account"].as_str().expect("account").to_string();
    // The Keychain refuses the delete: the file is saved without the entry, the account stays listed.
    with_keys(Denied, || turn_off(&root, "acme-tools").expect("off"));
    assert!(vault.has(&account));
    assert_eq!(file_of(&root)["retired"], json!([account]));
    // Tidying up later deletes it, without reading any key.
    with_keys(vault.clone(), || sweep(&root, None).expect("tidies"));
    assert!(!vault.has(&account));
    assert!(file_of(&root).get("retired").is_none());
}

#[test]
fn an_approval_that_is_not_saved_leaves_no_key_behind() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let vault = Vault::default();
    // The approvals file can't be written: a folder stands where it goes, and setting it aside fails too
    // (a folder that isn't empty is where it would go).
    let file = root.probe_approvals_file();
    fs::create_dir_all(&file).expect("mkdir");
    fs::create_dir_all(file.with_file_name("probe-approvals.json.bad").join("full")).expect("mkdir");
    with_keys(vault.clone(), || {
        let request = read_one(&root, "acme-tools").expect("request");
        let confirmed = request.confirm(&request.initial_grants(), Proof::TouchId).expect("a yes");
        assert!(matches!(approve(&root, &request, &confirmed, NOW), Err(ApproveError::Store(_))));
    });
    assert!(vault.0.lock().expect("vault").is_empty());
}

#[test]
fn an_unreadable_key_leaves_everything_off_and_deletes_nothing() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    install(&root, "quiet-one", ACME);
    with_keys(fixed(1), || approve_all(&root, "acme-tools"));
    let stored = fs::read_to_string(root.probe_approvals_file()).expect("read");
    let copy = root.probes_approved_dir().join("acme-tools");
    assert!(copy.exists());

    with_keys(Denied, || {
        let listing = list(&root).expect("lists");
        assert_eq!(listing.key_problem, Some(KeyProblem::Unreadable));
        for row in &listing.entries {
            assert_eq!(row.state, ProbeState::Off, "{}", row.folder);
            assert!(!row.unsealed, "an unreadable key says nothing about the entry itself");
        }
        assert!(listing.entries.iter().any(|row| row.folder == "acme-tools" && row.approved));
        assert_eq!(runnable(&root, "acme-tools").err(), Some(NotRunnable::KeyUnavailable));

        // Approving another Probe is refused before anything is copied or written.
        let request = read_one(&root, "quiet-one").expect("request");
        let confirmed = request.confirm(&request.initial_grants(), Proof::TouchId).expect("a yes");
        assert!(matches!(approve(&root, &request, &confirmed, NOW), Err(ApproveError::Key(_))));
        assert!(!root.probes_approved_dir().join("quiet-one").exists());
        assert!(fs::read_dir(root.probes_approved_dir()).expect("dir").count() == 1);
    });
    assert_eq!(fs::read_to_string(root.probe_approvals_file()).expect("read"), stored, "not a byte changed");
    assert!(copy.exists());
    with_keys(fixed(1), || assert_eq!(state(&root, "acme-tools"), ProbeState::On));
}

#[test]
fn turning_off_and_tidying_never_read_the_key() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    install(&root, "quiet-one", ACME);
    with_keys(fixed(1), || {
        approve_all(&root, "acme-tools");
        approve_all(&root, "quiet-one");
    });
    // A copy nothing owns, and an entry nobody sealed (its copy is kept: whose it is does not depend on the seal).
    fs::create_dir_all(root.probes_approved_dir().join("leftover")).expect("mkdir");
    let mut file = file_of(&root);
    file["probes"]["quiet-one"]["seal"] = json!("");
    write_file(&root, &file);
    with_keys(Forbidden, || {
        sweep(&root, None).expect("tidies");
        assert!(!root.probes_approved_dir().join("leftover").exists());
        assert!(root.probes_approved_dir().join("acme-tools").exists());
        assert!(root.probes_approved_dir().join("quiet-one").exists());

        // Forgetting what is gone keeps the seals of the rest as they are.
        let present: BTreeSet<String> = ["acme-tools".to_string()].into();
        let sealed = file_of(&root)["probes"]["acme-tools"]["seal"].clone();
        sweep(&root, Some(&present)).expect("forgets");
        assert!(file_of(&root)["probes"].get("quiet-one").is_none());
        assert_eq!(file_of(&root)["probes"]["acme-tools"]["seal"], sealed);

        turn_off(&root, "acme-tools").expect("off");
        assert!(file_of(&root)["probes"].as_object().expect("probes").is_empty());
    });
    // Turning off one of two leaves the other's seal valid.
    with_keys(fixed(1), || approve_all(&root, "acme-tools"));
    with_keys(fixed(1), || approve_all(&root, "quiet-one"));
    with_keys(Forbidden, || turn_off(&root, "acme-tools").expect("off"));
    with_keys(fixed(1), || assert_eq!(state(&root, "quiet-one"), ProbeState::On));
}

#[test]
fn the_development_keys_are_files_beside_the_approvals_one_per_approval() {
    let (_tmp, root) = setup();
    install(&root, "acme-tools", ACME);
    let keys = root.base().join(PURPOSE.dev_key_file);
    assert!(list(&root).is_ok() && !keys.exists(), "listing makes no key");
    approve_all(&root, "acme-tools");
    let account = file_of(&root)["probes"]["acme-tools"]["account"].as_str().expect("account").to_string();
    assert_eq!(fs::read(keys.join(&account)).expect("a key").len(), 32);
    assert_eq!(state(&root, "acme-tools"), ProbeState::On, "read back through the system's source");
    turn_off(&root, "acme-tools").expect("off");
    assert!(!keys.join(&account).exists(), "withdrawn, its key file is gone");
}
