//! Where the host program is, in core's words (`ipc::host_binary`), and the override tests and
//! development builds give it.

use std::path::PathBuf;

use wormhole_core::ipc::host_binary::{HostProgram, host_program};

/// The program a new host starts, or why there is none.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum HostingProgram {
    Found {
        path: String,
    },
    /// The app runs from a disk image or a translocated copy: a host started from there would outlive it.
    DisposableCopy,
    /// None beside the app.
    Missing,
}

impl From<&HostProgram> for HostingProgram {
    fn from(program: &HostProgram) -> Self {
        match program {
            HostProgram::Found(path) => Self::Found { path: path.display().to_string() },
            HostProgram::DisposableCopy => Self::DisposableCopy,
            HostProgram::Missing => Self::Missing,
        }
    }
}

/// `explicit` (an executable file the caller names: a test's build of the host) or core's rule, which
/// looks beside the app's own executable and nowhere else. The release library ignores
/// `WORMHOLE_HOST_BIN`, so the override is an argument here, not an environment variable; the library
/// refuses to run outside an isolated data folder, which is what keeps it from reaching a real host.
pub(crate) fn locate(explicit: Option<&str>) -> HostProgram {
    let Some(explicit) = explicit.filter(|path| !path.is_empty()) else { return host_program() };
    let path = PathBuf::from(explicit);
    let runnable = std::fs::metadata(&path)
        .is_ok_and(|meta| meta.is_file() && wormhole_core::platform::fs::is_executable(&meta));
    if runnable { HostProgram::Found(path) } else { HostProgram::Missing }
}
