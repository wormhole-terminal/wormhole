//! `wormhole-host --id <id> --dir <hosts dir>`: see the library.

fn main() -> std::process::ExitCode {
    wormhole_host::main(std::env::args_os())
}
