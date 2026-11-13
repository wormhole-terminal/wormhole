//! A control request made from inside the process, for the control folder's `control [--from <tab>] <words…>`
//! (the GPUI app's `devctl/control.rs`): the words are read by core's parser as `Wormhole <words…>` is, the request is
//! handed to the same handler the socket's requests reach, and its reply comes back as the one JSON line a
//! client prints with `--json`. Test tooling: the app's own control folder is the only caller.

use wormhole_core::ipc::control::printable::json_line;
use wormhole_core::ipc::control::{Parsed, Request, help, help_json, parse};
use wormhole_core::ipc::{Answer, Peer};
use wormhole_core::platform::paths;

use super::HookServer;

#[uniffi::export]
impl HookServer {
    /// Answer `words` as a request from tab `from` (`WORMHOLE_TAB_ID`; none: a shell outside the app). A reply that
    /// is a stream, a read, a wait or a run is not carried out here: the line says so.
    pub fn control_line(&self, words: Vec<String>, from: Option<String>) -> String {
        let cwd = std::env::current_dir().unwrap_or_default();
        let home = paths::home();
        let cli = match parse(&words, &cwd, home.as_deref()) {
            Ok(Parsed::Run(cli)) => cli,
            Ok(Parsed::Help) => return help(),
            Ok(Parsed::HelpJson) => return help_json(),
            Err(mistake) => return format!("error: {mistake}"),
        };
        let request = Request::new(cli.action, from);
        match super::control::answer(&self.core, request, &Peer::default()) {
            Answer::Reply(reply) => json_line(&reply),
            Answer::Read(_) => "not carried here: a read goes over the socket".into(),
            Answer::Wait(_) => "not carried here: a wait or run goes over the socket".into(),
            Answer::Stream { reply, .. } => format!("{} (the stream is not carried here)", json_line(&reply)),
            Answer::Ask(_) => "not carried here: the request waits for the person's approval".into(),
        }
    }
}
