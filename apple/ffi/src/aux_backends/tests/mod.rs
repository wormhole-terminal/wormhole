//! The backends against real core behaviour, over temp folders: a fake hosts folder with a stand-in host
//! process, throwaway projects, a loopback update feed and a scratch install folder. Never the person's data,
//! never a real swap of the running app, never the network beyond 127.0.0.1.

mod access;
mod first_run;
mod hosts;
mod imports;
mod licenses;
mod trust;
mod update_fixture;
mod updater;
