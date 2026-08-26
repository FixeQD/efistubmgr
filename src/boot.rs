use efivar::boot::{BootEntry, BootEntryAttributes, EFIHardDrive, FilePath, FilePathList};
use efivar::{Error as EfiError, VarManager};
use eros::Context;

use crate::die;
use crate::metadata;
use crate::nvram::try_list_generations;

/// Build BootEntry without disk access.
pub fn build_boot_entry(
    hard_drive: EFIHardDrive,
    loader_path: &str,
    description: &str,
    cmdline: &str,
    timestamp: i64,
) -> BootEntry {
    try_build_boot_entry(hard_drive, loader_path, description, cmdline, timestamp)
        .unwrap_or_else(|e| die(format!("building boot entry: {e}")))
}

pub fn try_build_boot_entry(
    hard_drive: EFIHardDrive,
    loader_path: &str,
    description: &str,
    cmdline: &str,
    timestamp: i64,
) -> eros::Result<BootEntry> {
    if loader_path.is_empty() {
        return Err(eros::error!("loader_path must not be empty"));
    }
    if description.is_empty() {
        return Err(eros::error!("description must not be empty"));
    }
    Ok(BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: description.to_string(),
        file_path_list: Some(FilePathList {
            file_path: FilePath {
                path: loader_path.to_string(),
            },
            hard_drive,
        }),
        optional_data: metadata::encode(timestamp, cmdline),
    })
}

/// Create logic using injected EFIHardDrive.
pub fn cmd_create_with_hard_drive(
    mgr: &mut dyn VarManager,
    hard_drive: EFIHardDrive,
    loader_path: &str,
    description: &str,
    cmdline: &str,
    timestamp: i64,
) -> u16 {
    try_cmd_create_with_hard_drive(
        mgr,
        hard_drive,
        loader_path,
        description,
        cmdline,
        timestamp,
    )
    .unwrap_or_else(|e| die(format!("creating boot entry: {e}")))
}

pub fn try_cmd_create_with_hard_drive(
    mgr: &mut dyn VarManager,
    hard_drive: EFIHardDrive,
    loader_path: &str,
    description: &str,
    cmdline: &str,
    timestamp: i64,
) -> eros::Result<u16> {
    let entry = try_build_boot_entry(hard_drive, loader_path, description, cmdline, timestamp)?;
    let id = crate::nvram::try_find_free_id(&*mgr)?;
    mgr.add_boot_entry(id, entry)
        .with_context(|| format!("creating boot entry Boot{id:04X}"))?;

    let known_ids: Vec<u16> = try_list_generations(&*mgr)?
        .into_iter()
        .map(|g| g.id)
        .collect();

    let mut order = match mgr.get_boot_order() {
        Ok(order) => order,
        Err(EfiError::VarNotFound { .. }) => Vec::new(),
        Err(e) => return Err(eros::error!(e).context("reading BootOrder")),
    };
    order.retain(|x| !known_ids.contains(x));
    order.splice(0..0, known_ids);
    mgr.set_boot_order(order).context("writing BootOrder")?;

    Ok(id)
}
