//! Recorded sessions (`scripts/vt-record.sh`) as corpora: a folder of `<name>.raw` files with an
//! optional `<name>.meta` holding `cols=` and `rows=`.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use super::Corpus;

/// The `*.raw` files of `dir`, sorted by path.
fn raw_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut raws: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "raw"))
        .collect();
    raws.sort();
    raws
}

fn name_of(path: &Path) -> Option<String> {
    Some(path.file_stem()?.to_string_lossy().into_owned())
}

/// The names of the recordings in `dir`, without reading them (for `--list`).
pub fn recorded_names(dir: &Path) -> Vec<String> {
    raw_files(dir).iter().filter_map(|p| name_of(p)).collect()
}

/// The recordings in `dir` as corpora, the size from their `.meta` (100x30 without). With
/// `only`, just the ones with these names, so a run that asks for one does not read them all.
pub fn recorded(dir: &Path, only: Option<&[String]>) -> Vec<Corpus> {
    raw_files(dir)
        .into_iter()
        .filter_map(|path| {
            let name = name_of(&path)?;
            if only.is_some_and(|names| !names.contains(&name)) {
                return None;
            }
            let bytes = std::fs::read(&path).ok()?;
            let meta = std::fs::read_to_string(path.with_extension("meta")).unwrap_or_default();
            let field = |key: &str| {
                meta.lines()
                    .find_map(|l| l.strip_prefix(key)?.strip_prefix('=')?.trim().parse::<usize>().ok())
            };
            Some(Corpus {
                name: Cow::Owned(name),
                description: "recorded with scripts/vt-record.sh",
                cols: field("cols").unwrap_or(100),
                rows: field("rows").unwrap_or(30),
                bytes,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch folder named for this test and process, removed afterwards by name.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Scratch {
            let dir = std::env::temp_dir().join(format!("vtbench-{test}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch folder");
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn recordings_are_listed_by_name_and_chosen_by_name() {
        let dir = Scratch::new("recorded-names");
        std::fs::write(dir.0.join("vim.raw"), b"\x1b[2Jhi").expect("write");
        std::fs::write(dir.0.join("vim.meta"), "cols=80\nrows=24\n").expect("write");
        std::fs::write(dir.0.join("less.raw"), b"x").expect("write");
        std::fs::write(dir.0.join("notes.txt"), b"not a recording").expect("write");

        assert_eq!(recorded_names(&dir.0), ["less", "vim"]);

        let all = recorded(&dir.0, None);
        assert_eq!(all.iter().map(|c| c.name.as_ref()).collect::<Vec<_>>(), ["less", "vim"]);
        assert_eq!((all[0].cols, all[0].rows), (100, 30), "no .meta: the default size");

        let chosen = recorded(&dir.0, Some(&["vim".to_string()]));
        assert_eq!(chosen.len(), 1);
        assert_eq!((chosen[0].name.as_ref(), chosen[0].cols, chosen[0].rows), ("vim", 80, 24));
        assert_eq!(chosen[0].bytes, b"\x1b[2Jhi");
    }
}
