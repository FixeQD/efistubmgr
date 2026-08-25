use efivar::boot::{BootEntry, BootEntryAttributes, EFIHardDrive, FilePath, FilePathList};
use efivar::{Error as EfiError, VarManager};
use eros::Context;

use crate::die;
use crate::nvram::try_list_generations;
use crate::DESC_PREFIX;

/// Build BootEntry without disk access.
pub fn build_boot_entry(
    hard_drive: EFIHardDrive,
    loader_path: &str,
    optional_data: &str,
    timestamp: i64,
) -> BootEntry {
    try_build_boot_entry(hard_drive, loader_path, optional_data, timestamp)
        .unwrap_or_else(|e| die(format!("building boot entry: {e}")))
}

pub fn try_build_boot_entry(
    hard_drive: EFIHardDrive,
    loader_path: &str,
    optional_data: &str,
    timestamp: i64,
) -> eros::Result<BootEntry> {
    if loader_path.is_empty() {
        return Err(eros::error!("loader_path must not be empty"));
    }
    Ok(BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: format!("{DESC_PREFIX}{timestamp}"),
        file_path_list: Some(FilePathList {
            file_path: FilePath {
                path: loader_path.to_string(),
            },
            hard_drive,
        }),
        optional_data: optional_data
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect(),
    })
}

/// Create logic using injected EFIHardDrive.
pub fn cmd_create_with_hard_drive(
    mgr: &mut dyn VarManager,
    hard_drive: EFIHardDrive,
    loader_path: &str,
    optional_data: &str,
    timestamp: i64,
) -> u16 {
    try_cmd_create_with_hard_drive(mgr, hard_drive, loader_path, optional_data, timestamp)
        .unwrap_or_else(|e| die(format!("creating boot entry: {e}")))
}

pub fn try_cmd_create_with_hard_drive(
    mgr: &mut dyn VarManager,
    hard_drive: EFIHardDrive,
    loader_path: &str,
    optional_data: &str,
    timestamp: i64,
) -> eros::Result<u16> {
    let entry = try_build_boot_entry(hard_drive, loader_path, optional_data, timestamp)?;
    let id = crate::nvram::try_find_free_id(&*mgr)?;
    mgr.add_boot_entry(id, entry)
        .with_context(|| format!("creating boot entry Boot{id:04X}"))?;

    let known_ids: Vec<u16> = try_list_generations(&*mgr)?.into_iter().map(|g| g.id).collect();

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