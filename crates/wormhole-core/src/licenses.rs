//! The open-source software Wormhole is built from: each crate's declared license
//! and the license texts that shipped in its package.
//!
//! The data is `assets/licenses.json`, written by `scripts/gen-licenses.py` (run it after
//! changing dependencies). Identical texts are stored once and shared by index.

use std::sync::OnceLock;

use anyhow::{Result, ensure};
use serde::Deserialize;

const BUNDLED: &str = include_str!("../assets/licenses.json");

#[derive(Debug, Default, Deserialize)]
pub struct Licenses {
    texts: Vec<String>,
    crates: Vec<Crate>,
}

#[derive(Debug, Deserialize)]
pub struct Crate {
    pub name: String,
    pub version: String,
    /// The SPDX expression from the crate's `Cargo.toml`.
    pub license: Option<String>,
    pub repository: Option<String>,
    /// Indices into [`Licenses::texts`].
    texts: Vec<usize>,
}

impl Crate {
    /// `name version`, as listed.
    pub fn title(&self) -> String {
        format!("{} {}", self.name, self.version)
    }
}

impl Licenses {
    /// The list compiled into the app. Empty (and logged) if the bundled file is broken,
    /// which a test rules out.
    pub fn bundled() -> &'static Licenses {
        static BUNDLED_LICENSES: OnceLock<Licenses> = OnceLock::new();
        BUNDLED_LICENSES.get_or_init(|| {
            Self::parse(BUNDLED).unwrap_or_else(|err| {
                tracing::error!("the bundled license list is unreadable: {err:#}");
                Licenses::default()
            })
        })
    }

    pub fn parse(json: &str) -> Result<Licenses> {
        let licenses: Licenses = serde_json::from_str(json)?;
        for krate in &licenses.crates {
            ensure!(
                krate.texts.iter().all(|&ix| ix < licenses.texts.len()),
                "{} points at a license text that does not exist",
                krate.title()
            );
        }
        Ok(licenses)
    }

    pub fn crates(&self) -> &[Crate] {
        &self.crates
    }

    /// The license files of `krate`, in the order they were found. Empty when the crate's
    /// package ships none (its declared license is all we know).
    pub fn texts_of<'a>(&'a self, krate: &'a Crate) -> impl Iterator<Item = &'a str> {
        krate.texts.iter().map(|&ix| self.texts[ix].as_str())
    }

    /// Indices into [`Self::crates`] of the crates matching `query`: every word of it must
    /// appear in the crate's name, version or license, ignoring case. An empty query
    /// matches all.
    pub fn search(&self, query: &str) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.crates
            .iter()
            .enumerate()
            .filter(|(_, krate)| {
                let haystack = format!(
                    "{} {} {}",
                    krate.name,
                    krate.version,
                    krate.license.as_deref().unwrap_or_default()
                )
                .to_lowercase();
                words.iter().all(|word| haystack.contains(word))
            })
            .map(|(ix, _)| ix)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::Licenses;

    const SAMPLE: &str = r#"{"texts": ["MIT text", "Apache text"], "crates": [
        {"name": "alpha", "version": "1.0.0", "license": "MIT OR Apache-2.0", "repository": null, "texts": [0, 1]},
        {"name": "beta-core", "version": "2.3.0", "license": "Zlib", "repository": "https://example.com/beta", "texts": []}
    ]}"#;

    #[test]
    fn bundled_list_is_readable_and_sorted() {
        let licenses = Licenses::bundled();
        assert!(licenses.crates().len() > 100, "the bundled list is empty or broken");
        let names: Vec<_> = licenses.crates().iter().map(|c| c.name.to_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        // Parsing checked every text index; make sure texts really come out.
        assert!(licenses.crates().iter().any(|c| licenses.texts_of(c).next().is_some()));
    }

    #[test]
    fn bundled_list_matches_the_lockfile() {
        // A dependency upgrade without `scripts/gen-licenses.py` leaves an entry for a version
        // the app is no longer built from.
        let lock: toml::Table = include_str!("../../../Cargo.lock").parse().expect("Cargo.lock parses");
        let locked: HashSet<(String, String)> = lock["package"]
            .as_array()
            .expect("packages")
            .iter()
            .filter_map(|p| {
                Some((p.get("name")?.as_str()?.to_string(), p.get("version")?.as_str()?.to_string()))
            })
            .collect();
        let stale: Vec<String> = Licenses::bundled()
            .crates()
            .iter()
            .filter(|c| !locked.contains(&(c.name.clone(), c.version.clone())))
            .map(|c| c.title())
            .collect();
        assert!(stale.is_empty(), "run scripts/gen-licenses.py; not in Cargo.lock any more: {stale:?}");
    }

    /// The other direction: a dependency added without the generator ships without its
    /// license listed. Cargo.lock alone can't tell a shipped crate from a dev-only or
    /// other-platform one, so this asks `cargo metadata` what the generator asks it, for this
    /// Mac's target (its crates are fetched, so it works offline); on Linux the macOS crates may
    /// not be, and the stale check above still runs.
    #[cfg(target_os = "macos")]
    #[test]
    fn bundled_list_has_every_crate_the_app_is_built_from() {
        let target =
            if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" } else { "x86_64-apple-darwin" };
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let output = std::process::Command::new(cargo)
            .args(["metadata", "--format-version", "1", "--locked", "--offline", "--filter-platform", target])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("cargo runs");
        assert!(output.status.success(), "cargo metadata: {}", String::from_utf8_lossy(&output.stderr));
        let meta: serde_json::Value = serde_json::from_slice(&output.stdout).expect("metadata is JSON");
        let shipped = shipped(&meta);
        assert!(shipped.len() > 100, "the dependency graph came out empty");
        let listed: HashSet<(String, String)> =
            Licenses::bundled().crates().iter().map(|c| (c.name.clone(), c.version.clone())).collect();
        let mut missing: Vec<String> =
            shipped.difference(&listed).map(|(name, version)| format!("{name} {version}")).collect();
        missing.sort();
        assert!(missing.is_empty(), "run scripts/gen-licenses.py; not in licenses.json: {missing:?}");
    }

    /// What `scripts/gen-licenses.py` lists: the packages reachable from the workspace crates
    /// through normal dependencies (not dev- or build-only ones), as (name, version).
    #[cfg(target_os = "macos")]
    fn shipped(meta: &serde_json::Value) -> HashSet<(String, String)> {
        use std::collections::HashMap;
        let packages: HashMap<&str, (String, String)> = meta["packages"]
            .as_array()
            .expect("packages")
            .iter()
            .filter_map(|p| {
                let id = p["id"].as_str()?;
                Some((id, (p["name"].as_str()?.to_string(), p["version"].as_str()?.to_string())))
            })
            .collect();
        let nodes: HashMap<&str, &serde_json::Value> = meta["resolve"]["nodes"]
            .as_array()
            .expect("resolve")
            .iter()
            .filter_map(|node| Some((node["id"].as_str()?, node)))
            .collect();
        let members: Vec<&str> = meta["workspace_members"]
            .as_array()
            .expect("members")
            .iter()
            .filter_map(|m| m.as_str())
            .collect();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack = members.clone();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(deps) = nodes.get(id).and_then(|node| node["deps"].as_array()) else { continue };
            for dep in deps {
                let normal = dep["dep_kinds"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind["kind"].is_null()));
                if let Some(pkg) = dep["pkg"].as_str().filter(|_| normal) {
                    stack.push(pkg);
                }
            }
        }
        seen.into_iter()
            .filter(|id| !members.contains(id))
            .filter_map(|id| packages.get(id).cloned())
            .collect()
    }

    #[test]
    fn crates_carry_their_texts() {
        let licenses = Licenses::parse(SAMPLE).expect("sample parses");
        let alpha = &licenses.crates()[0];
        assert_eq!(alpha.title(), "alpha 1.0.0");
        assert_eq!(licenses.texts_of(alpha).collect::<Vec<_>>(), ["MIT text", "Apache text"]);
        assert_eq!(licenses.texts_of(&licenses.crates()[1]).count(), 0);
    }

    #[test]
    fn a_dangling_text_index_is_an_error() {
        let broken = SAMPLE.replace("[0, 1]", "[0, 7]");
        let err = Licenses::parse(&broken).expect_err("index 7 does not exist");
        assert!(err.to_string().contains("alpha 1.0.0"), "{err}");
    }

    #[test]
    fn search_matches_every_word_in_name_version_or_license() {
        let licenses = Licenses::parse(SAMPLE).expect("sample parses");
        assert_eq!(licenses.search(""), [0, 1]);
        assert_eq!(licenses.search("ALPHA"), [0]);
        assert_eq!(licenses.search("zlib"), [1]);
        assert_eq!(licenses.search("mit apache"), [0]);
        assert_eq!(licenses.search("2.3"), [1]);
        assert_eq!(licenses.search("mit zlib"), Vec::<usize>::new());
        assert!(licenses.search("nothing like this").is_empty());
    }
}
