//! `platform::default_app` where there is no default terminal to be.

pub fn own_bundle() -> Option<String> {
    None
}

pub fn runner_for(_: &str) -> Option<String> {
    None
}

pub fn set_runner_for(_: &str, _: &str) -> anyhow::Result<()> {
    anyhow::bail!("only macOS has a default terminal")
}
