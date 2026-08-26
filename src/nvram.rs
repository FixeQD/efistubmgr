use efivar::boot::BootEntry;
use efivar::efi::Variable;
use efivar::VarManager;
use eros::Context;

use crate::die;
use crate::metadata;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Generation {
    pub id: u16,
    pub ts: i64,
}

pub fn list_generations(mgr: &dyn VarManager) -> Vec<Generation> {
    try_list_generations(mgr).unwrap_or_else(|e| die(format!("enumerating NVRAM variables: {e}")))
}

pub fn try_list_generations(mgr: &dyn VarManager) -> eros::Result<Vec<Generation>> {
    let vars = mgr.get_all_vars().context("enumerating NVRAM variables")?;
    let mut gens: Vec<Generation> = vars
        .filter_map(|var| {
            let id = var.boot_var_id()?;
            match BootEntry::read(mgr, &var) {
                Ok(entry) => {
                    // only entries with hidden metadata (created via metadata::encode)
                    let ts = metadata::entry_timestamp(&entry)?;
                    Some(Generation { id, ts })
                }
                Err(e) => {
                    eprintln!(
                        "{}: warning: failed to parse boot entry {var:?}: {e}",
                        env!("CARGO_PKG_NAME")
                    );
                    None
                }
            }
        })
        .collect();

    gens.sort_by_key(|b| std::cmp::Reverse(b.ts));
    Ok(gens)
}

/// Format generations without printing.
pub fn list_generations_formatted(mgr: &dyn VarManager) -> Vec<String> {
    try_list_generations_formatted(mgr)
        .unwrap_or_else(|e| die(format!("formatting generations: {e}")))
}

pub fn try_list_generations_formatted(mgr: &dyn VarManager) -> eros::Result<Vec<String>> {
    Ok(try_list_generations(mgr)?
        .into_iter()
        .map(|g| format!("{:04X} {}", g.id, g.ts))
        .collect())
}

pub fn find_free_id(mgr: &dyn VarManager) -> u16 {
    try_find_free_id(mgr).unwrap_or_else(|e| die(format!("finding free id: {e}")))
}

pub fn try_find_free_id(mgr: &dyn VarManager) -> eros::Result<u16> {
    for id in 0u16..=0xFFFF {
        let var = Variable::new(&format!("Boot{id:04X}"));
        let exists = mgr
            .exists(&var)
            .with_context(|| format!("checking Boot{id:04X}"))?;
        if !exists {
            return Ok(id);
        }
    }
    Err(eros::error!("no free Boot#### slot left"))
}
