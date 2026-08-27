//! Serializes create/delete/list against concurrent efistubmgr invocations with an flock on a shared runtime lock file

use std::fs::{File, OpenOptions};
use std::path::Path;

use eros::Context;

const LOCK_PATH: &str = "/run/efistubmgr.lock";

pub(crate) fn open_lock_file(path: &Path) -> eros::Result<File> {
    Ok(OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .with_context(|| format!("opening lock file {}", path.display()))?)
}

pub(crate) fn exclusive_at(path: &Path) -> eros::Result<File> {
    let f = open_lock_file(path)?;
    f.lock()
        .with_context(|| format!("locking {} exclusively", path.display()))?;
    Ok(f)
}

pub(crate) fn shared_at(path: &Path) -> eros::Result<File> {
    let f = open_lock_file(path)?;
    f.lock_shared()
        .with_context(|| format!("locking {} (shared)", path.display()))?;
    Ok(f)
}

/// Exclusive lock for commands that mutate NVRAM
/// Blocks until any other exclusive or shared holder releases.
pub fn acquire_exclusive() -> eros::Result<File> {
    exclusive_at(Path::new(LOCK_PATH))
}

/// Shared lock for read-only commands
pub fn acquire_shared() -> eros::Result<File> {
    shared_at(Path::new(LOCK_PATH))
}
