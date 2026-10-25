//! AppKit's side of `platform::trash`: `NSFileManager` moves the file, as the Finder would.

use std::path::Path;

use objc2_foundation::{NSFileManager, NSString, NSURL};

pub fn move_to_trash(path: &Path) -> anyhow::Result<()> {
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, None)
        .map_err(|error| anyhow::anyhow!("{}", error.localizedDescription()))
}
