use std::path::Path;

/// Where macOS keeps its own version. `sw_vers` is a front end for this file.
const SYSTEM_VERSION_PLIST: &str = "/System/Library/CoreServices/SystemVersion.plist";

pub(crate) fn version_text() -> Option<String> {
    version_in(Path::new(SYSTEM_VERSION_PLIST)).or_else(version_from_sw_vers)
}

/// `ProductVersion` in a `SystemVersion.plist`.
fn version_in(path: &Path) -> Option<String> {
    let value = plist::Value::from_file(path).ok()?;
    Some(value.as_dictionary()?.get("ProductVersion")?.as_string()?.to_owned())
}

fn version_from_sw_vers() -> Option<String> {
    let out = std::process::Command::new("/usr/bin/sw_vers").arg("-productVersion").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_plists_product_version_is_read_and_a_bad_file_gives_none() {
        let dir = tempfile::tempdir().expect("temp");
        let write = |body: &str| {
            let path = dir.path().join("SystemVersion.plist");
            std::fs::write(&path, body).expect("write");
            path
        };
        let plist = |version: &str| {
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict>\
                 <key>ProductVersion</key><string>{version}</string></dict></plist>"
            )
        };
        assert_eq!(version_in(&write(&plist("15.0.1"))).as_deref(), Some("15.0.1"));
        assert_eq!(version_in(&write("not a plist")), None);
        assert_eq!(version_in(&write("<plist version=\"1.0\"><dict/></plist>")), None);
        assert_eq!(version_in(&dir.path().join("missing.plist")), None);
    }
}
