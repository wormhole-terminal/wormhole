//! The server over a real socket: a client that connects to the data folder's socket as `wh` does, the
//! workspace and its terminals (`/bin/cat`) behind it, the desk that asks the person, on a temp data folder
//! of the experiment's own. Never the person's data, never a running Wormhole.

mod access;
mod control;
mod drive;
mod hooks;
mod probes;
mod profiles;
mod streams;
mod support;
mod welcome;
mod windows;
