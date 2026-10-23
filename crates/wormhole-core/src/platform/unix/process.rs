use std::io;
use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::process::{Command, ExitStatus};

/// Replace this process with the command; only returns if that failed.
pub(crate) fn exec(command: &mut Command) -> io::Error {
    command.exec()
}

pub(crate) fn own_process_group(command: &mut Command) -> &mut Command {
    command.process_group(0)
}

/// An exit status from the raw wait status the system reports (a code is in its high byte).
pub(crate) fn exit_status_from_raw(raw: i32) -> ExitStatus {
    ExitStatus::from_raw(raw)
}
