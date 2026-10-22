//! One consistent read of a `.wormhole/` folder.
//!
//! Each file is read once, with the same capped, root-bound reads as the single-file loaders, and
//! everything else is derived from those bytes: the parsed config, actions, prompts and layout,
//! and the trust fingerprint. What the approval dialog shows, what the trust check compares and
//! what an action runs are therefore the same content. Reading the disk again for the check (as
//! trust did before) let a `git checkout` between check and run make them differ.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::actions::{self, Action};
use super::features::root_key;
use super::trust::{self, Input, TrustRequest};
use super::{
    ACTIONS_FILE, CONFIG_FILE, Feature, FeatureStanding, LAYOUT_FILE, Layout, MAX_JSON_BYTES,
    MAX_PROMPT_BYTES, PROMPTS_DIR, ProjectConfig, ProjectError, ProjectFolder, ProjectWarning, Prompt,
    THEME_FILE, TrustStatus, check_relative, config_from_value, parse_json, parse_layout, prompts,
};
use crate::storage::fsutil::FileStamp;

/// A loaded project folder. Cheap to query: nothing here touches the disk after [`Self::load`].
#[derive(Debug)]
pub struct ProjectSnapshot {
    folder: ProjectFolder,
    /// The trust store's key for `folder.root`, computed once (it canonicalizes).
    trust_key: String,
    config: Result<Option<ProjectConfig>, ProjectError>,
    theme: Result<Option<serde_json::Value>, ProjectError>,
    actions: Result<Option<Vec<Action>>, ProjectError>,
    /// What was set aside in a file that loaded (an action's key that can't be used).
    warnings: Vec<ProjectWarning>,
    prompts: Result<Option<Vec<Prompt>>, ProjectError>,
    layout: Result<Option<Layout>, ProjectError>,
    /// The text of every file a prompt action names (by its `file`), as fingerprinted: what the
    /// action pastes. Files that couldn't be read are missing (the actions then failed to load).
    action_prompts: BTreeMap<String, String>,
    /// Present trust-relevant files, for the approval dialog.
    trusted_files: Vec<PathBuf>,
    fingerprint: String,
    /// The fingerprint of the same files before `config.json` was taken by what it says, which
    /// approvals given back then are checked against (see `TrustStore::status_of`).
    legacy_fingerprint: String,
    /// Every path read (or looked for), stamped just before: if they all still look the same,
    /// so does the snapshot.
    read: Vec<(PathBuf, Option<FileStamp>)>,
}

/// A file as read for the snapshot: text (`None` if absent) or why it couldn't be read.
type Read = Result<Option<String>, ProjectError>;

impl ProjectSnapshot {
    pub fn load(folder: &ProjectFolder) -> Self {
        let mut stamped = Vec::new();
        let mut stamp = |path: &Path| stamped.push((path.to_path_buf(), FileStamp::of(path)));
        // The folder itself: replacing or relinking it is a change even when no file inside is.
        stamp(&folder.dir);
        let mut read = |path: &Path, cap: u64| {
            stamp(path);
            folder.read_text(path, cap)
        };
        let config_read = read(&folder.path(CONFIG_FILE), MAX_JSON_BYTES);
        let actions_read = read(&folder.path(ACTIONS_FILE), MAX_JSON_BYTES);
        let layout_read = read(&folder.path(LAYOUT_FILE), MAX_JSON_BYTES);
        let theme_read = read(&folder.path(THEME_FILE), MAX_JSON_BYTES);

        // The shape of the actions decides which prompt files are trust inputs too.
        let actions_path = folder.path(ACTIONS_FILE);
        let parsed_actions = match &actions_read {
            Ok(Some(text)) => Some(parse_json::<Vec<Action>>(&actions_path, text)),
            _ => None,
        };
        // The one parse of `config.json`: the settings, environment and colour come from it, and so
        // does what its fingerprint covers.
        let config_path = folder.path(CONFIG_FILE);
        let parsed_config = match &config_read {
            Ok(Some(text)) => Some(parse_json::<serde_json::Value>(&config_path, text)),
            _ => None,
        };
        let mut prompt_reads: BTreeMap<String, Read> = BTreeMap::new();
        if let Some(Ok(list)) = &parsed_actions {
            for file in actions::prompt_files(list) {
                // A path leaving `prompts/` is never read; validation below reports it.
                if check_relative(file).is_ok() {
                    let path = folder.path(PROMPTS_DIR).join(file);
                    prompt_reads.insert(file.to_string(), read(&path, MAX_PROMPT_BYTES));
                }
            }
        }

        let mut inputs: Vec<(String, Input)> = vec![
            (CONFIG_FILE.to_string(), trust::config_input(&config_read, parsed_config.as_ref())),
            (ACTIONS_FILE.to_string(), Input::of(&actions_read)),
            (LAYOUT_FILE.to_string(), Input::of(&layout_read)),
        ];
        inputs.extend(
            prompt_reads.iter().map(|(file, read)| (format!("{PROMPTS_DIR}/{file}"), Input::of(read))),
        );
        let fingerprint = trust::fingerprint_of(
            trust::FINGERPRINT_VERSION,
            inputs.iter().map(|(name, input)| (name.as_str(), *input)),
        );
        // The same files as approvals were given for before `config.json` was taken by what it
        // says: its bytes.
        inputs[0].1 = Input::of(&config_read);
        let legacy_fingerprint = trust::fingerprint_of(
            trust::LEGACY_VERSION,
            inputs.iter().map(|(name, input)| (name.as_str(), *input)),
        );

        let mut trusted_files: Vec<PathBuf> = [CONFIG_FILE, ACTIONS_FILE, LAYOUT_FILE]
            .into_iter()
            .zip([&config_read, &actions_read, &layout_read])
            .filter(|(_, read)| matches!(read, Ok(Some(_))))
            .map(|(name, _)| folder.path(name))
            .collect();
        trusted_files.extend(
            prompt_reads
                .iter()
                .filter(|(_, read)| matches!(read, Ok(Some(_))))
                .map(|(file, _)| folder.path(PROMPTS_DIR).join(file)),
        );

        let mut warnings = Vec::new();
        let actions = match (actions_read, parsed_actions) {
            (Err(err), _) => Err(err),
            (Ok(_), Some(Err(err))) => Err(err),
            (Ok(_), Some(Ok(mut list))) => {
                let prompt_ok = |file: &str| match prompt_reads.get(file) {
                    Some(Ok(Some(_))) => Ok(()),
                    Some(Err(err)) => Err(err.reason()),
                    Some(Ok(None)) | None => Err("does not exist".to_string()),
                };
                match actions::validate(&mut list, &prompt_ok) {
                    Ok(warned) => {
                        warnings = warned;
                        Ok(Some(list))
                    }
                    Err(m) => Err(ProjectError::invalid(&actions_path, m)),
                }
            }
            (Ok(_), None) => Ok(None),
        };
        let action_prompts = prompt_reads
            .into_iter()
            .filter_map(|(file, read)| read.ok().flatten().map(|text| (file, text)))
            .collect();

        let prompts = prompts::load_dir(folder, &mut stamp);
        let config = match (config_read, parsed_config) {
            (Err(err), _) => Err(err),
            (Ok(_), Some(Err(err))) => Err(err),
            (Ok(_), Some(Ok(value))) => config_from_value(&config_path, value).map(Some),
            (Ok(_), None) => Ok(None),
        };
        if let Ok(Some(config)) = &config {
            warnings.extend(config.color_error.iter().map(|why| ProjectWarning::Color { why: why.clone() }));
            warnings
                .extend(config.lock_error.iter().map(|why| ProjectWarning::ColorLock { why: why.clone() }));
        }
        Self {
            trust_key: root_key(&folder.root),
            config,
            theme: parse_read(theme_read, &folder.path(THEME_FILE), parse_json),
            actions,
            warnings,
            prompts,
            layout: parse_read(layout_read, &folder.path(LAYOUT_FILE), parse_layout),
            action_prompts,
            trusted_files,
            fingerprint,
            legacy_fingerprint,
            read: stamped,
            folder: folder.clone(),
        }
    }

    /// Whether every file this snapshot was read from still looks the same. Metadata only (one
    /// or two `stat`s per file), so it is cheap enough to poll.
    pub fn is_current(&self) -> bool {
        self.read.iter().all(|(path, stamp)| FileStamp::of(path) == *stamp)
    }

    /// A fresh snapshot if anything it was read from changed, else `None` (nothing is read).
    pub fn reload_if_changed(&self) -> Option<Self> {
        (!self.is_current()).then(|| Self::load(&self.folder))
    }

    pub fn folder(&self) -> &ProjectFolder {
        &self.folder
    }

    pub fn root(&self) -> &Path {
        &self.folder.root
    }

    /// SHA-256 (hex) of everything that can make the app run or type something (see
    /// [`super::fingerprint`]), over exactly the bytes this snapshot parsed.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// The fingerprint approvals had before `config.json` was taken by what it says (its bytes,
    /// colour included). An approval equal to it is for exactly this content.
    pub(super) fn legacy_fingerprint(&self) -> &str {
        &self.legacy_fingerprint
    }

    pub(super) fn trust_key(&self) -> &str {
        &self.trust_key
    }

    pub fn config(&self) -> Option<&ProjectConfig> {
        self.config.as_ref().ok()?.as_ref()
    }

    /// Why `config.json` is there but didn't load (it isn't JSON, or `env` or `shell` is wrong).
    pub fn config_error(&self) -> Option<&ProjectError> {
        self.config.as_ref().err()
    }

    pub fn theme_json(&self) -> Option<&serde_json::Value> {
        self.theme.as_ref().ok()?.as_ref()
    }

    /// Empty when there is no `actions.json` or it doesn't load.
    pub fn actions(&self) -> &[Action] {
        self.actions.as_ref().ok().and_then(Option::as_ref).map_or(&[], Vec::as_slice)
    }

    pub fn prompts(&self) -> &[Prompt] {
        self.prompts.as_ref().ok().and_then(Option::as_ref).map_or(&[], Vec::as_slice)
    }

    pub fn layout(&self) -> Option<&Layout> {
        self.layout.as_ref().ok()?.as_ref()
    }

    /// The text a prompt action with this `file` pastes: the bytes that were fingerprinted, not
    /// whatever the file holds by the time the action runs.
    pub fn action_prompt(&self, file: &str) -> Option<&str> {
        self.action_prompts.get(file).map(String::as_str)
    }

    /// Whether the files behind `feature` are present (as of this snapshot). Notes are synced
    /// separately and always count as possible.
    pub fn has(&self, feature: Feature) -> bool {
        match feature {
            Feature::Settings => matches!(self.config, Ok(Some(_)) | Err(_)),
            Feature::Theme => matches!(self.theme, Ok(Some(_)) | Err(_)),
            Feature::Actions => matches!(self.actions, Ok(Some(_)) | Err(_)),
            Feature::Prompts => matches!(self.prompts, Ok(Some(_)) | Err(_)),
            Feature::Layout => matches!(self.layout, Ok(Some(_)) | Err(_)),
            Feature::Notes => self.folder.has(Feature::Notes),
        }
    }

    /// Whether `feature`'s file sets anything that is used: a `config.json` with every key
    /// commented out (what Create project folder writes), an `actions.json` of `[]`, a layout
    /// without terminals or an empty prompts folder set nothing. A file that doesn't load counts as
    /// setting something, since what it would set can't be told (its error is listed on its own).
    /// Notes count as set when the file is there.
    pub fn sets_anything(&self, feature: Feature) -> bool {
        match feature {
            Feature::Settings => match &self.config {
                Ok(config) => config.as_ref().is_some_and(|c| {
                    !c.settings.is_empty()
                        || !c.env.is_empty()
                        || c.shell.is_some()
                        || c.color.is_some()
                        || c.color_locked
                }),
                Err(_) => true,
            },
            Feature::Theme => match &self.theme {
                Ok(theme) => theme.as_ref().is_some_and(|t| match t {
                    serde_json::Value::Object(map) => !map.is_empty(),
                    other => !other.is_null(),
                }),
                Err(_) => true,
            },
            Feature::Actions => match &self.actions {
                Ok(actions) => actions.as_ref().is_some_and(|list| !list.is_empty()),
                Err(_) => true,
            },
            Feature::Prompts => match &self.prompts {
                Ok(prompts) => prompts.as_ref().is_some_and(|list| !list.is_empty()),
                Err(_) => true,
            },
            Feature::Layout => match &self.layout {
                Ok(layout) => layout.as_ref().is_some_and(|l| !l.sessions.is_empty()),
                Err(_) => true,
            },
            Feature::Notes => self.has(feature),
        }
    }

    /// Whether something in `feature`'s file only takes effect once the project is trusted. Not all
    /// of what [`Self::sets_anything`] counts does: `project_color` only paints, so a `config.json`
    /// with just that is used at once. All the rest of Settings is held back, `font_size` as much as
    /// `env` (trust gates the whole feature), and so is every action and layout terminal.
    pub fn needs_trust(&self, feature: Feature) -> bool {
        if !feature.requires_trust() {
            return false;
        }
        match feature {
            Feature::Settings => match &self.config {
                Ok(config) => config
                    .as_ref()
                    .is_some_and(|c| !c.settings.is_empty() || !c.env.is_empty() || c.shell.is_some()),
                Err(_) => true,
            },
            _ => self.sets_anything(feature),
        }
    }

    /// The features something in the folder needs trust for, in the order of [`Feature::ALL`].
    pub fn needing_trust(&self) -> Vec<Feature> {
        Feature::ALL.into_iter().filter(|feature| self.needs_trust(*feature)).collect()
    }

    /// How `feature` stands in this project when its trust is `status`: what the line under its
    /// switch says (see [`FeatureStanding`]).
    pub fn standing(&self, feature: Feature, status: TrustStatus) -> FeatureStanding {
        if !self.has(feature) {
            FeatureStanding::Missing
        } else if !self.sets_anything(feature) {
            FeatureStanding::Empty
        } else if self.needs_trust(feature) && status != TrustStatus::Trusted {
            FeatureStanding::NeedsTrust
        } else {
            FeatureStanding::Ready
        }
    }

    /// What loaded but was set aside in part (an action's key that is taken), for the Project
    /// tab; none of it holds up trust.
    pub fn warnings(&self) -> &[ProjectWarning] {
        &self.warnings
    }

    /// Problems loading the folder's files, each naming its file.
    pub fn errors(&self) -> impl Iterator<Item = &ProjectError> {
        let config = self.config.as_ref().err();
        let theme = self.theme.as_ref().err();
        let actions = self.actions.as_ref().err();
        let prompts = self.prompts.as_ref().err();
        let layout = self.layout.as_ref().err();
        [config, actions, prompts, layout, theme].into_iter().flatten()
    }

    /// What trusting this exact content would enable. `None` while a trust-relevant file fails
    /// to load, since an invalid file can't be shown (or used) faithfully.
    pub fn trust_request(&self) -> Option<TrustRequest> {
        if self.config.is_err() || self.actions.is_err() || self.layout.is_err() {
            return None;
        }
        Some(trust::request_for(self, self.trusted_files.clone()))
    }

    /// [`Self::trust_request`], or the first error that prevents it.
    pub fn into_trust_request(self) -> Result<TrustRequest, ProjectError> {
        if let Some(request) = self.trust_request() {
            return Ok(request);
        }
        match (self.config, self.actions, self.layout) {
            (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => Err(err),
            _ => Err(ProjectError::invalid(&self.folder.dir, "can't be described")),
        }
    }
}

fn parse_read<T>(
    read: Read,
    path: &Path,
    parse: impl FnOnce(&Path, &str) -> Result<T, ProjectError>,
) -> Result<Option<T>, ProjectError> {
    match read? {
        Some(text) => parse(path, &text).map(Some),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{project, reads_during};
    use super::super::{ActionKind, NOTES_FILE};
    use super::*;
    use std::fs;

    const CONFIG: &str = r#"{"font_size": 13, "env": {"A": "1"}, "shell": "/bin/zsh"}"#;
    const ACTIONS: &str = r#"[
        {"id": "t", "title": "Test", "type": "run", "command": "cargo test"},
        {"id": "r", "title": "Review", "type": "prompt", "file": "review.md", "submit": true}
    ]"#;
    const LAYOUT: &str = r#"{"sessions": [{"name": "Claude", "run": "claude"}]}"#;

    #[test]
    fn parses_the_same_as_the_single_file_loaders() {
        let (_tmp, p) = project(&[
            (CONFIG_FILE, CONFIG),
            (ACTIONS_FILE, ACTIONS),
            (LAYOUT_FILE, LAYOUT),
            (THEME_FILE, r#"{"name": "Dusk"}"#),
            ("prompts/review.md", "# Review\nLook at the diff."),
        ]);
        let snapshot = ProjectSnapshot::load(&p);
        assert_eq!(snapshot.config(), p.config().expect("ok").as_ref());
        assert_eq!(snapshot.actions(), p.actions().expect("ok").unwrap_or_default().as_slice());
        assert_eq!(snapshot.layout(), p.layout().expect("ok").as_ref());
        assert_eq!(snapshot.theme_json(), p.theme_json().expect("ok").as_ref());
        assert_eq!(snapshot.prompts(), p.prompts().expect("ok").unwrap_or_default().as_slice());
        // The raw file is what a prompt action pastes (front matter and heading included).
        assert_eq!(snapshot.action_prompt("review.md"), Some("# Review\nLook at the diff."));
        assert_eq!(snapshot.fingerprint(), super::super::fingerprint(&p));
        assert_eq!(snapshot.errors().count(), 0);
        for feature in Feature::ALL {
            assert_eq!(snapshot.has(feature), p.has(feature), "{feature:?}");
        }
        let request = snapshot.trust_request().expect("request");
        assert_eq!(request.files.len(), 4, "{:?}", request.files);
        assert_eq!(request.root, p.root);
    }

    #[test]
    fn an_empty_folder_has_nothing() {
        let (_tmp, p) = project(&[]);
        let snapshot = ProjectSnapshot::load(&p);
        assert!(
            snapshot.config().is_none() && snapshot.layout().is_none() && snapshot.theme_json().is_none()
        );
        assert!(snapshot.actions().is_empty() && snapshot.prompts().is_empty());
        assert!(Feature::ALL.iter().all(|f| !snapshot.has(*f)));
        let request = snapshot.trust_request().expect("request");
        assert!(request.commands.is_empty() && request.files.is_empty());
        std::fs::write(p.path(NOTES_FILE), "").expect("write");
        assert!(ProjectSnapshot::load(&p).has(Feature::Notes));
    }

    #[test]
    fn broken_files_are_reported_and_block_trust() {
        let (_tmp, p) = project(&[(CONFIG_FILE, CONFIG), (ACTIONS_FILE, "[{"), (THEME_FILE, "{")]);
        let snapshot = ProjectSnapshot::load(&p);
        let files: Vec<_> = snapshot.errors().map(|e| e.path().file_name().map(|n| n.to_owned())).collect();
        assert_eq!(files.len(), 2);
        assert!(snapshot.has(Feature::Actions), "a broken file is still there");
        assert!(snapshot.actions().is_empty());
        assert!(snapshot.config().is_some(), "the other files still load");
        assert!(snapshot.trust_request().is_none());
        let err = ProjectSnapshot::load(&p).into_trust_request().expect_err("broken");
        assert!(err.path().ends_with(ACTIONS_FILE));
        // A broken theme alone doesn't stop trust: it can't run anything.
        let (_tmp, p) = project(&[(ACTIONS_FILE, "[]"), (THEME_FILE, "{")]);
        assert!(ProjectSnapshot::load(&p).trust_request().is_some());
    }

    /// An action's `file` that leaves `prompts/` is reported, and never read or hashed.
    #[test]
    fn a_prompt_path_leaving_the_prompts_folder_is_not_read() {
        let actions = r#"[{"id": "x", "title": "X", "type": "prompt", "file": "../config.json"}]"#;
        let (_tmp, p) = project(&[(ACTIONS_FILE, actions), (CONFIG_FILE, CONFIG)]);
        let snapshot = ProjectSnapshot::load(&p);
        assert!(snapshot.actions().is_empty());
        assert!(snapshot.errors().any(|e| e.to_string().contains("relative path inside the project")));
        assert_eq!(snapshot.action_prompt("../config.json"), None);
        assert_eq!(snapshot.trust_request(), None);
    }

    #[test]
    fn chain_steps_reach_their_prompt_files() {
        let actions = r#"[{"id": "c", "title": "C", "type": "chain",
            "steps": [{"type": "prompt", "file": "a.md", "target": "new_session"}]}]"#;
        let (_tmp, p) = project(&[(ACTIONS_FILE, actions), ("prompts/a.md", "hello")]);
        let snapshot = ProjectSnapshot::load(&p);
        assert!(matches!(&snapshot.actions()[0].kind, ActionKind::Chain { .. }));
        assert_eq!(snapshot.action_prompt("a.md"), Some("hello"));
    }

    /// The app polls every project every 2 s; an unchanged folder must cost
    /// metadata lookups only, not reads, parses and two SHA-256 passes.
    #[test]
    fn an_unchanged_folder_is_not_read_again() {
        let (_tmp, p) = project(&[
            (CONFIG_FILE, CONFIG),
            (ACTIONS_FILE, ACTIONS),
            ("prompts/review.md", "Review."),
            ("prompts/other.md", "Other."),
        ]);
        let snapshot = ProjectSnapshot::load(&p);
        let reads = reads_during(|| {
            for _ in 0..10 {
                assert!(snapshot.reload_if_changed().is_none());
            }
        });
        assert_eq!(reads, 0);
        assert!(reads_during(|| drop(ProjectSnapshot::load(&p))) > 0, "the counter works");
    }

    #[test]
    fn every_kind_of_change_is_noticed() {
        let (_tmp, p) =
            project(&[(ACTIONS_FILE, ACTIONS), ("prompts/review.md", "Review."), ("prompts/sub/x.md", "x")]);
        type Change = Box<dyn Fn(&ProjectFolder)>;
        let changes: [(&str, Change); 7] = [
            ("a new file", Box::new(|p| fs::write(p.path(CONFIG_FILE), CONFIG).expect("write"))),
            ("an edited file", Box::new(|p| fs::write(p.path(CONFIG_FILE), "{}").expect("write"))),
            ("a deleted file", Box::new(|p| fs::remove_file(p.path(CONFIG_FILE)).expect("rm"))),
            ("a new prompt", Box::new(|p| fs::write(p.path("prompts/new.md"), "n").expect("write"))),
            ("an edited prompt", Box::new(|p| fs::write(p.path("prompts/review.md"), "R!").expect("write"))),
            (
                "a prompt only an action names",
                Box::new(|p| {
                    let actions = ACTIONS.replace("review.md", "sub/x.md");
                    fs::write(p.path(ACTIONS_FILE), actions).expect("write");
                }),
            ),
            ("its text", Box::new(|p| fs::write(p.path("prompts/sub/x.md"), "rm -rf ~").expect("write"))),
        ];
        let mut snapshot = ProjectSnapshot::load(&p);
        for (what, change) in changes {
            change(&p);
            let fresh = snapshot.reload_if_changed().unwrap_or_else(|| panic!("{what} went unnoticed"));
            assert!(fresh.is_current());
            snapshot = fresh;
        }
        assert_eq!(snapshot.action_prompt("sub/x.md"), Some("rm -rf ~"));
    }

    #[cfg(unix)]
    #[test]
    fn relinking_is_noticed() {
        let (_tmp, p) = project(&[("prompts/a.md", "a"), ("prompts/b.md", "bbb")]);
        std::os::unix::fs::symlink("prompts/a.md", p.path(LAYOUT_FILE)).expect("symlink");
        let snapshot = ProjectSnapshot::load(&p);
        fs::remove_file(p.path(LAYOUT_FILE)).expect("rm");
        std::os::unix::fs::symlink("prompts/b.md", p.path(LAYOUT_FILE)).expect("symlink");
        assert!(!snapshot.is_current());
    }

    /// What a Project tab row can say about a file: a new project's `config.json` has every key
    /// commented out, so it sets nothing and trust has nothing to apply.
    #[test]
    fn a_file_that_sets_nothing_is_empty_not_waiting_for_trust() {
        let (_tmp, p) = project(&[]);
        let fresh = p.root.join("fresh");
        fs::create_dir(&fresh).expect("mkdir");
        super::super::init(&fresh).expect("created");
        let template = ProjectFolder::open(&fresh).expect("a project").snapshot();
        assert!(template.has(Feature::Settings), "the file is there");
        assert!(!template.sets_anything(Feature::Settings), "but every key is a comment");
        assert!(template.needing_trust().is_empty());
        for status in [TrustStatus::Untrusted, TrustStatus::Changed, TrustStatus::Trusted] {
            assert_eq!(template.standing(Feature::Settings, status), FeatureStanding::Empty, "{status:?}");
        }
        assert_eq!(template.standing(Feature::Actions, TrustStatus::Untrusted), FeatureStanding::Missing);

        for (file, text) in [
            (ACTIONS_FILE, "[]"),
            (LAYOUT_FILE, r#"{"sessions": []}"#),
            (THEME_FILE, "{}"),
            (CONFIG_FILE, "{}"),
        ] {
            let (_tmp, p) = project(&[(file, text)]);
            let snapshot = p.snapshot();
            let feature = Feature::ALL.into_iter().find(|f| f.file_name() == file).expect("feature");
            assert_eq!(snapshot.standing(feature, TrustStatus::Untrusted), FeatureStanding::Empty, "{file}");
            assert!(snapshot.needing_trust().is_empty(), "{file}");
        }
        let (_tmp, p) = project(&[("prompts/.keep", "")]);
        assert_eq!(p.snapshot().standing(Feature::Prompts, TrustStatus::Untrusted), FeatureStanding::Empty);
    }

    #[test]
    fn what_is_set_waits_for_trust_until_the_project_has_it() {
        let cases = [
            (CONFIG_FILE, r#"{"font_size": 15}"#, Feature::Settings),
            (CONFIG_FILE, r#"{"env": {"A": "1"}}"#, Feature::Settings),
            (CONFIG_FILE, r#"{"shell": "/bin/zsh"}"#, Feature::Settings),
            (
                ACTIONS_FILE,
                r#"[{"id": "t", "title": "T", "type": "run", "command": "make"}]"#,
                Feature::Actions,
            ),
            (LAYOUT_FILE, r#"{"sessions": [{"name": "Shell"}]}"#, Feature::Layout),
        ];
        for (file, text, feature) in cases {
            let (_tmp, p) = project(&[(file, text)]);
            let snapshot = p.snapshot();
            assert!(snapshot.sets_anything(feature) && snapshot.needs_trust(feature), "{text}");
            assert_eq!(snapshot.needing_trust(), [feature], "{text}");
            for status in [TrustStatus::Untrusted, TrustStatus::Changed] {
                assert_eq!(snapshot.standing(feature, status), FeatureStanding::NeedsTrust, "{text}");
            }
            assert_eq!(snapshot.standing(feature, TrustStatus::Trusted), FeatureStanding::Ready, "{text}");
        }
    }

    /// `project_color` only paints, so a `config.json` with just that is used at once, which
    /// also means there is nothing to ask trust for.
    #[test]
    fn a_colour_alone_is_set_but_needs_no_trust() {
        let (_tmp, p) = project(&[(CONFIG_FILE, r##"{"project_color": "#2bb3a3"}"##)]);
        let snapshot = p.snapshot();
        assert!(snapshot.sets_anything(Feature::Settings));
        assert!(!snapshot.needs_trust(Feature::Settings) && snapshot.needing_trust().is_empty());
        assert_eq!(snapshot.standing(Feature::Settings, TrustStatus::Untrusted), FeatureStanding::Ready);
        // A name is a colour too; a word that isn't one is set aside: it sets nothing.
        let (_tmp, p) = project(&[(CONFIG_FILE, r#"{"project_color": "teal"}"#)]);
        let snapshot = p.snapshot();
        assert!(snapshot.sets_anything(Feature::Settings) && !snapshot.needs_trust(Feature::Settings));
        let (_tmp, p) = project(&[(CONFIG_FILE, r#"{"project_color": "brown"}"#)]);
        assert!(!p.snapshot().sets_anything(Feature::Settings));
        // The lock paints nothing but is set, and asks for no trust either.
        let (_tmp, p) = project(&[(CONFIG_FILE, r#"{"project_color_locked": true}"#)]);
        let snapshot = p.snapshot();
        assert!(snapshot.sets_anything(Feature::Settings) && !snapshot.needs_trust(Feature::Settings));
    }

    #[test]
    fn a_config_that_does_not_load_says_why() {
        let (_tmp, p) = project(&[(CONFIG_FILE, "{ \"font_size\": ")]);
        let snapshot = p.snapshot();
        assert!(snapshot.has(Feature::Settings) && snapshot.config().is_none());
        assert!(snapshot.config_error().is_some_and(|err| err.path().ends_with(CONFIG_FILE)));
        let (_tmp, p) = project(&[(CONFIG_FILE, "{}")]);
        assert!(p.snapshot().config_error().is_none());
        let (_tmp, p) = project(&[]);
        assert!(p.snapshot().config_error().is_none(), "no file is no error");
    }

    /// Theme and prompts never ask for trust; notes count as there when the file is.
    #[test]
    fn the_features_that_need_no_trust_never_wait_for_it() {
        let (_tmp, p) =
            project(&[(THEME_FILE, r#"{"name": "Dusk"}"#), ("prompts/a.md", "A"), (NOTES_FILE, "")]);
        let snapshot = p.snapshot();
        for feature in [Feature::Theme, Feature::Prompts, Feature::Notes] {
            assert!(snapshot.sets_anything(feature), "{feature:?}");
            assert_eq!(snapshot.standing(feature, TrustStatus::Untrusted), FeatureStanding::Ready);
        }
        assert!(snapshot.needing_trust().is_empty());
    }

    /// A file that doesn't load can't be said to set nothing, so it keeps asking for trust (the
    /// problem itself is listed on its own).
    #[test]
    fn a_file_that_does_not_load_counts_as_setting_something() {
        let (_tmp, p) = project(&[(CONFIG_FILE, "{"), (ACTIONS_FILE, "[{")]);
        let snapshot = p.snapshot();
        assert_eq!(snapshot.needing_trust(), [Feature::Settings, Feature::Actions]);
        assert_eq!(snapshot.standing(Feature::Actions, TrustStatus::Untrusted), FeatureStanding::NeedsTrust);
    }

    /// `cargo test -p wormhole-core --lib -- --ignored --nocapture snapshot_cost`
    #[test]
    #[ignore = "timing, run by hand"]
    fn snapshot_cost() {
        let mut files = vec![(CONFIG_FILE, CONFIG.to_string()), (ACTIONS_FILE, ACTIONS.to_string())];
        files.push((LAYOUT_FILE, LAYOUT.to_string()));
        files.push(("prompts/review.md", "Review the diff. ".repeat(100)));
        let names: Vec<String> = (0..20).map(|i| format!("prompts/p{i}.md")).collect();
        files.extend(names.iter().map(|n| (n.as_str(), "A prompt. ".repeat(200))));
        let refs: Vec<(&str, &str)> = files.iter().map(|(n, t)| (*n, t.as_str())).collect();
        let (_tmp, p) = project(&refs);
        let snapshot = ProjectSnapshot::load(&p);
        let time = |f: &dyn Fn()| {
            let start = std::time::Instant::now();
            for _ in 0..1000 {
                f();
            }
            start.elapsed() / 1000
        };
        let load = time(&|| drop(ProjectSnapshot::load(&p)));
        let check = time(&|| assert!(snapshot.is_current()));
        // What a refresh cost before: the loaders, then `describe_trust_request` (fingerprint +
        // the loaders again), then `TrustStore::status` (fingerprint again).
        let hash_trusted_files = || {
            use sha2::Digest;
            for name in [CONFIG_FILE, ACTIONS_FILE, LAYOUT_FILE] {
                let _ = sha2::Sha256::digest(fs::read(p.path(name)).unwrap_or_default());
            }
        };
        let before = time(&|| {
            let _ = (p.config(), p.actions(), p.prompts(), p.layout());
            hash_trusted_files();
            let _ = (p.config(), p.actions(), p.layout());
            hash_trusted_files();
        });
        eprintln!("full load {load:?}, unchanged check {check:?}, old refresh path ~{before:?}");
    }
}
