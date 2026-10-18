//! The command line: `wormhole-host --id <id> --dir <hosts dir>`, and nothing else. What to run
//! never comes this way (argv is visible in `ps`): it comes in `Spawn`, over the socket.

use std::ffi::OsString;
use std::path::PathBuf;

use wormhole_term::hosted::meta::valid_id;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    /// 12 characters from `[a-z0-9]`.
    pub id: String,
    /// The hosts directory, `<data dir>/hosts`, absolute.
    pub dir: PathBuf,
}

impl Args {
    /// The data directory the hosts directory is in.
    pub fn data_dir(&self) -> PathBuf {
        self.dir.parent().map_or_else(|| self.dir.clone(), PathBuf::from)
    }
}

/// Read the arguments after the program's name.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
    let mut args = args.into_iter();
    let (mut id, mut dir) = (None, None);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--id") => {
                let value = args.next().ok_or("--id needs a value")?;
                id = Some(value.into_string().map_err(|_| "--id is not text")?);
            }
            Some("--dir") => dir = Some(PathBuf::from(args.next().ok_or("--dir needs a value")?)),
            _ => return Err(format!("unknown argument {arg:?}")),
        }
    }
    let id = id.ok_or("--id is missing")?;
    if !valid_id(&id) {
        return Err("--id must be 12 characters from a-z and 0-9".into());
    }
    let dir = dir.ok_or("--dir is missing")?;
    if !dir.is_absolute() {
        return Err("--dir must be an absolute path".into());
    }
    Ok(Args { id, dir })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Args, String> {
        parse(list.iter().map(OsString::from))
    }

    #[test]
    fn the_id_and_the_dir_are_all_it_takes() {
        let parsed = args(&["--id", "abcdefghij12", "--dir", "/data/hosts"]).expect("parses");
        assert_eq!(parsed.id, "abcdefghij12");
        assert_eq!(parsed.data_dir(), PathBuf::from("/data"));
        assert_eq!(
            args(&["--dir", "/d", "--id", "abcdefghij12"]).expect("any order").dir,
            PathBuf::from("/d")
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in [
            &["--id", "abcdefghij12"][..],
            &["--dir", "/data/hosts"],
            &["--id", "short", "--dir", "/d"],
            &["--id", "abcdefghij12", "--dir", "relative"],
            &["--id", "abcdefghij12", "--dir", "/d", "--shell", "/bin/sh"],
            &["--id"],
        ] {
            assert!(args(bad).is_err(), "{bad:?}");
        }
    }
}
