//! Paths a manifest names inside the Probe's own folder.

use super::Findings;
use crate::probe::read::ProbeFolder;

/// A manifest path made plain: no `..`, no root, no empty or `.` part in the middle, no NUL or
/// backslash; one leading `./` is allowed and removed. The result is how [`ProbeFolder::file`] spells
/// it, and the kind of path that cannot lead out of the folder.
pub(crate) fn inside_folder(path: &str) -> Result<String, &'static str> {
    let path = path.strip_prefix("./").unwrap_or(path);
    if path.is_empty() {
        return Err("is empty");
    }
    if path.starts_with('/') {
        return Err("is absolute; it must be inside the Probe's folder");
    }
    if path.contains('\0') || path.contains('\\') || crate::probe::hidden::has_hidden(path) {
        return Err("holds a control or invisible character or a backslash");
    }
    for part in path.split('/') {
        match part {
            ".." => return Err("goes up out of the Probe's folder (`..`)"),
            "" | "." => return Err("has an empty or `.` part"),
            _ => {}
        }
    }
    Ok(path.to_string())
}

/// A manifest path that must name a file of the folder; the file when it does. Findings use
/// `code` for a path that is not plain and `missing` for one that names nothing.
pub(crate) fn file_of<'a>(
    folder: &'a ProbeFolder,
    path: &str,
    field: &str,
    codes: (&'static str, &'static str),
    out: &mut Findings,
) -> Option<&'a crate::probe::read::ProbeFile> {
    match inside_folder(path) {
        Err(why) => {
            out.error(codes.0, field, format!("`{path}` {why}"));
            None
        }
        Ok(plain) => {
            let file = folder.file(&plain);
            if file.is_none() {
                out.error(codes.1, field, format!("`{path}` is not a file of the Probe's folder"));
            }
            file
        }
    }
}

/// Themes and icon themes: the files the existing loaders will read must be there.
pub(super) fn check_contributed_files(folder: &ProbeFolder, out: &mut Findings) {
    let c = &folder.manifest.contributes;
    for (kind, list) in [("themes", &c.themes), ("icon_themes", &c.icon_themes)] {
        for (ix, item) in list.iter().enumerate() {
            let field = format!("contributes.{kind}[{ix}].path");
            file_of(folder, &item.path, &field, ("contributed-path", "contributed-file-missing"), out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_path_is_kept_and_a_leading_dot_slash_goes() {
        assert_eq!(inside_folder("scripts/deploy.sh").as_deref(), Ok("scripts/deploy.sh"));
        assert_eq!(inside_folder("./scripts/deploy.sh").as_deref(), Ok("scripts/deploy.sh"));
    }

    #[test]
    fn nothing_that_could_leave_the_folder_is_plain() {
        for bad in
            ["", "./", "/etc/passwd", "../x", "a/../b", "a//b", "a/./b", "a\\b", "a\0b", "a\nb", "./../x"]
        {
            assert!(inside_folder(bad).is_err(), "{bad:?}");
        }
    }
}
