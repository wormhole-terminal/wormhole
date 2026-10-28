//! The Files tab's commands: its tree as text, the folder it is of, which folders are open, and the
//! path menu and pins. The work is the feature's (`features::files::devctl`).

use std::path::PathBuf;

use anyhow::{Result, bail};

use super::Ctx;
use crate::features::files::devctl as files;

/// A folder named in a command: absolute, since the tree is of a folder anywhere.
fn folder(arg: &str) -> Result<PathBuf> {
    let path = PathBuf::from(arg.trim());
    if !path.is_absolute() {
        bail!("expected an absolute path, got {:?}", arg.trim());
    }
    Ok(path)
}

/// `files`: the tab's state as text.
pub(super) fn state(ctx: &mut Ctx, _: &str) -> Result<String> {
    files::describe(ctx.workspace, ctx.cx)
}

/// `files-root <path>`: the tree is of that folder, as if chosen in the path menu.
pub(super) fn root(ctx: &mut Ctx, arg: &str) -> Result<String> {
    files::root(ctx.workspace, &folder(arg)?, ctx.cx)?;
    Ok(String::new())
}

/// `files-follow`: Follow Terminal.
pub(super) fn follow(ctx: &mut Ctx, _: &str) -> Result<String> {
    files::follow(ctx.workspace, ctx.cx)?;
    Ok(String::new())
}

/// `files-expand <relative path>`: opens that folder and those above it.
pub(super) fn expand(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let relative = PathBuf::from(arg.trim());
    if relative.is_absolute() || relative.components().any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!("expected a relative path below the root, got {:?}", arg.trim());
    }
    files::expand(ctx.workspace, &relative, ctx.cx)?;
    Ok(String::new())
}

/// `files-menu [model]`: the path menu, drawn for `shot` (or only recorded, for `menu` and `menu-pick`).
pub(super) fn menu(ctx: &mut Ctx, arg: &str) -> Result<String> {
    files::menu(ctx.workspace, arg.trim() != "model", ctx.window, ctx.cx)?;
    Ok(String::new())
}

/// `files-pin <path>`: pins the folder, or unpins it.
pub(super) fn pin(ctx: &mut Ctx, arg: &str) -> Result<String> {
    files::pin(ctx.workspace, &folder(arg)?, ctx.cx)
}

/// A path below the tree's root named in a command: relative and plain.
fn relative(arg: &str) -> Result<PathBuf> {
    let path = PathBuf::from(arg.trim());
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!("expected a relative path below the root, got {:?}", arg.trim());
    }
    Ok(path)
}

/// `files-open <relative path>`: shows that file in the viewer.
pub(super) fn open(ctx: &mut Ctx, arg: &str) -> Result<String> {
    files::open(ctx.workspace, &relative(arg)?, ctx.cx)?;
    Ok(String::new())
}

/// `files-rename-start <relative path>`: opens the row's rename field, renames nothing.
pub(super) fn rename_start(ctx: &mut Ctx, arg: &str) -> Result<String> {
    files::rename_start(ctx.workspace, &relative(arg)?, ctx.window, ctx.cx)?;
    Ok(String::new())
}

/// `files-rename <relative path> <new name>`: the path has no space in it, the name is the rest of the line.
pub(super) fn rename(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (path, name) =
        arg.trim().split_once(' ').ok_or_else(|| anyhow::anyhow!("expected <path> <new name>"))?;
    files::rename(ctx.workspace, &relative(path)?, name, ctx.window, ctx.cx)?;
    Ok(String::new())
}
