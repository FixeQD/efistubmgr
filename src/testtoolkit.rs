//! testtoolkit - helpers for testing bootctl
//! Only mock NVRAM / boot simulation helpers.

use efivar::boot::{BootEntry, BootEntryAttributes, BootVarReader, BootVarWriter, EFIHardDrive, EFIHardDriveType, FilePath, FilePathList};
use efivar::efi::Variable;
use efivar::store::MemoryStore;
use efivar::VarManager;
use eros::Context;
use uuid::Uuid;

use crate::boot::try_build_boot_entry;
use crate::die;
use crate::nvram::{try_list_generations, Generation};
use crate::DESC_PREFIX;

pub fn try_dummy_hard_drive(partition_number: u32) -> eros::Result<EFIHardDrive> {
    let sig = Uuid::parse_str("12345678-1234-1234-1234-123456789abc").context("parsing dummy UUID")?;
    Ok(EFIHardDrive {
        partition_number,
        partition_start: 2048 + (partition_number as u64 * 1000),
        partition_size: 100_000,
        partition_sig: sig,
        format: 0x02,
        sig_type: EFIHardDriveType::Gpt,
    })
}

pub fn dummy_hard_drive(partition_number: u32) -> EFIHardDrive {
    try_dummy_hard_drive(partition_number).unwrap_or_else(|e| die(format!("dummy_hard_drive: {e}")))
}

pub fn try_dummy_hard_drive_with_sig(partition_number: u32, sig: Uuid) -> eros::Result<EFIHardDrive> {
    if sig.is_nil() {
        return Err(eros::error!("partition sig must not be nil"));
    }
    Ok(EFIHardDrive {
        partition_number,
        partition_start: 2048,
        partition_size: 50000,
        partition_sig: sig,
        format: 0x02,
        sig_type: EFIHardDriveType::Gpt,
    })
}

pub fn dummy_hard_drive_with_sig(partition_number: u32, sig: Uuid) -> EFIHardDrive {
    try_dummy_hard_drive_with_sig(partition_number, sig)
        .unwrap_or_else(|e| die(format!("dummy_hard_drive_with_sig: {e}")))
}

pub fn try_make_entry(ts: i64, partition_number: u32, loader: &str, optional: &str) -> eros::Result<BootEntry> {
    let hd = try_dummy_hard_drive(partition_number)?;
    Ok(BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: format!("{}{}", crate::DESC_PREFIX, ts),
        file_path_list: Some(FilePathList {
            file_path: FilePath { path: loader.to_string() },
            hard_drive: hd,
        }),
        optional_data: optional.encode_utf16().flat_map(|c| c.to_le_bytes()).collect(),
    })
}

pub fn make_entry(ts: i64, partition_number: u32, loader: &str, optional: &str) -> BootEntry {
    try_make_entry(ts, partition_number, loader, optional)
        .unwrap_or_else(|e| die(format!("make_entry: {e}")))
}

pub fn try_insert_entry(mgr: &mut MemoryStore, id: u16, ts: i64) -> eros::Result<()> {
    let entry = try_make_entry(ts, 1, "\\EFI\\Boot\\bootx64.efi", "")?;
    mgr.add_boot_entry(id, entry)
        .with_context(|| format!("inserting Entry entry Boot{id:04X} ts={ts}"))?;
    Ok(())
}

pub fn insert_entry(mgr: &mut MemoryStore, id: u16, ts: i64) {
    try_insert_entry(mgr, id, ts).unwrap_or_else(|e| die(format!("{e}")))
}

pub fn try_insert_entry_with_loader(mgr: &mut MemoryStore, id: u16, ts: i64, loader: &str, optional: &str) -> eros::Result<()> {
    let entry = try_make_entry(ts, 1, loader, optional)?;
    mgr.add_boot_entry(id, entry)
        .with_context(|| format!("inserting Entry entry Boot{id:04X} loader={loader:?}"))?;
    Ok(())
}

pub fn insert_entry_with_loader(mgr: &mut MemoryStore, id: u16, ts: i64, loader: &str, optional: &str) {
    try_insert_entry_with_loader(mgr, id, ts, loader, optional).unwrap_or_else(|e| die(format!("{e}")))
}

pub fn try_insert_other_entry(mgr: &mut MemoryStore, id: u16, desc: &str) -> eros::Result<()> {
    if desc.is_empty() {
        return Err(eros::error!("non-Entry description must not be empty"));
    }
    let entry = BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: desc.to_string(),
        file_path_list: Some(FilePathList {
            file_path: FilePath { path: "\\EFI\\Other\\boot.efi".to_string() },
            hard_drive: try_dummy_hard_drive(1)?,
        }),
        optional_data: vec![],
    };
    mgr.add_boot_entry(id, entry)
        .with_context(|| format!("inserting non-Entry entry Boot{id:04X} desc={desc:?}"))?;
    Ok(())
}

pub fn insert_other_entry(mgr: &mut MemoryStore, id: u16, desc: &str) {
    try_insert_other_entry(mgr, id, desc).unwrap_or_else(|e| die(format!("{e}")))
}

pub fn try_boot_order(mgr: &MemoryStore) -> eros::Result<Vec<u16>> {
    Ok(mgr.get_boot_order().context("reading BootOrder")?)
}

pub fn boot_order(mgr: &MemoryStore) -> Vec<u16> {
    try_boot_order(mgr).unwrap_or_default()
}

pub fn try_set_boot_order(mgr: &mut MemoryStore, order: Vec<u16>) -> eros::Result<()> {
    let order_clone = order.clone();
    mgr.set_boot_order(order)
        .with_context(|| format!("writing BootOrder {order_clone:?}"))?;
    Ok(())
}

pub fn set_boot_order(mgr: &mut MemoryStore, order: Vec<u16>) {
    try_set_boot_order(mgr, order).unwrap_or_else(|e| die(format!("{e}")))
}

pub fn try_simulate_boot(mgr: &dyn VarManager) -> eros::Result<Option<(u16, BootEntry)>> {
    let order = match mgr.get_boot_order() {
        Ok(o) => o,
        Err(efivar::Error::VarNotFound { .. }) => return Ok(None),
        Err(e) => return Err(eros::error!(e).context("reading BootOrder for simulate_boot")),
    };
    for id in order {
        let var = Variable::new(&format!("Boot{id:04X}"));
        match BootEntry::read(mgr, &var) {
            Ok(entry) if entry.attributes.contains(BootEntryAttributes::LOAD_OPTION_ACTIVE) => {
                return Ok(Some((id, entry)))
            }
            Ok(_) => continue,
            Err(e) => {
                eprintln!("{}: warning: failed to parse boot entry {var:?}: {e}", env!("CARGO_PKG_NAME"));
                continue;
            }
        }
    }
    Ok(None)
}

pub fn simulate_boot(mgr: &dyn VarManager) -> Option<(u16, BootEntry)> {
    try_simulate_boot(mgr).unwrap_or_else(|e| {
        eprintln!("{}: warning: simulate_boot failed: {e}", env!("CARGO_PKG_NAME"));
        None
    })
}

pub fn try_simulate_desc_boot(mgr: &dyn VarManager) -> eros::Result<Option<(u16, i64)>> {
    Ok(try_list_generations(mgr)?
        .into_iter()
        .next()
        .map(|g| (g.id, g.ts)))
}

pub fn simulate_desc_boot(mgr: &dyn VarManager) -> Option<(u16, i64)> {
    try_simulate_desc_boot(mgr).unwrap_or_else(|e| {
        eprintln!("{}: warning: simulate_desc_boot failed: {e}", env!("CARGO_PKG_NAME"));
        None
    })
}

#[cfg(test)]
#[path = "../tests/mod.rs"]
mod tests;
