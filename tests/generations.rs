use crate::nvram::{list_generations, list_generations_formatted};
use crate::testtoolkit::{dummy_hard_drive, insert_entry, insert_other_entry};
use efivar::VarWriter;
use efivar::boot::{BootEntry, BootEntryAttributes, FilePath, FilePathList};
use efivar::efi::{Variable, VariableFlags};
use efivar::store::MemoryStore;

#[test]
fn list_generations_empty() {
    let store = MemoryStore::new();
    assert!(list_generations(&store).is_empty());
}

#[test]
fn list_generations_filters_other() {
    let mut store = MemoryStore::new();
    insert_other_entry(&mut store, 0, "Windows Boot Manager");
    insert_other_entry(&mut store, 1, "ubuntu");
    insert_entry(&mut store, 2, 500);
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 1);
    assert_eq!(gens[0].id, 2);
    assert_eq!(gens[0].ts, 500);
}

#[test]
fn list_generations_sorts_newest_first() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 300);
    insert_entry(&mut store, 2, 200);
    let gens = list_generations(&store);
    assert_eq!(
        gens.iter().map(|g| g.ts).collect::<Vec<_>>(),
        vec![300, 200, 100]
    );
    assert_eq!(gens[0].id, 1);
    assert_eq!(gens[1].id, 2);
    assert_eq!(gens[2].id, 0);
}

#[test]
fn list_generations_ignores_unparsable_desc_prefix() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    // Entry without metadata must be ignored regardless of description
    let bad = BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: "NotATimestamp".to_string(),
        file_path_list: Some(FilePathList {
            file_path: FilePath {
                path: "\\EFI\\bad.efi".into(),
            },
            hard_drive: dummy_hard_drive(1),
        }),
        optional_data: vec![],
    };
    use efivar::boot::BootVarWriter;
    store.add_boot_entry(1, bad).unwrap();
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 1);
    assert_eq!(gens[0].id, 0);
}

#[test]
fn list_generations_ignores_corrupt_entries() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    // Write raw corrupt data for Boot0001
    store
        .write(
            &Variable::new("Boot0001"),
            VariableFlags::NON_VOLATILE,
            &[0xFF, 0x00],
        )
        .unwrap();
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 1);
    assert_eq!(gens[0].id, 0);
}

#[test]
fn list_generations_handles_missing_boot_var_id() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 10);
    store
        .write(
            &Variable::new("BootOrder"),
            VariableFlags::NON_VOLATILE,
            &[0, 0],
        )
        .unwrap();
    store
        .write(
            &Variable::new("BootNext"),
            VariableFlags::NON_VOLATILE,
            &[0],
        )
        .unwrap();
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 1);
}

#[test]
fn list_generations_duplicate_timestamps() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 100);
    insert_entry(&mut store, 2, 100);
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 3);
    let ids: Vec<u16> = gens.iter().map(|g| g.id).collect();
    assert!(ids.contains(&0) && ids.contains(&1) && ids.contains(&2));
}

#[test]
fn list_generations_negative_timestamps() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, -100);
    insert_entry(&mut store, 1, 0);
    insert_entry(&mut store, 2, -50);
    let gens = list_generations(&store);
    assert_eq!(gens[0].ts, 0);
    assert_eq!(gens[1].ts, -50);
    assert_eq!(gens[2].ts, -100);
}

#[test]
fn list_generations_trimmed_description() {
    // entries without hidden metadata are never listed, even if description looks numeric
    let mut store = MemoryStore::new();
    let entry = BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: "  123".to_string(),
        file_path_list: Some(FilePathList {
            file_path: FilePath {
                path: "\\EFI\\a.efi".into(),
            },
            hard_drive: dummy_hard_drive(1),
        }),
        optional_data: vec![],
    };
    use efivar::boot::BootVarWriter;
    store.add_boot_entry(0, entry).unwrap();
    let gens = list_generations(&store);
    assert!(
        gens.is_empty(),
        "description-only entries must not be listed"
    );
}

#[test]
fn list_generations_formatted_output() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0x1, 1000);
    insert_entry(&mut store, 0xA, 2000);
    let lines = list_generations_formatted(&store);
    assert_eq!(lines, vec!["000A 2000", "0001 1000"]);
}

#[test]
fn list_generations_does_not_include_hidden_filter() {
    // Hidden Entry entries should still be listed - our filter only checks description
    let mut store = MemoryStore::new();
    let mut entry = crate::testtoolkit::make_entry(100, 1, "\\EFI\\Boot\\boot.efi", "");
    entry.attributes = BootEntryAttributes::LOAD_OPTION_HIDDEN;
    use efivar::boot::BootVarWriter;
    store.add_boot_entry(0, entry).unwrap();
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 1); // we don't filter by attributes
}

#[test]
fn list_generations_with_only_corrupt_entries_is_empty() {
    let mut store = MemoryStore::new();
    for i in 0..5 {
        store
            .write(
                &Variable::new(&format!("Boot{i:04X}")),
                VariableFlags::NON_VOLATILE,
                &[0, 1, 2],
            )
            .unwrap();
    }
    assert!(list_generations(&store).is_empty());
}

#[test]
fn list_generations_ignores_boot_order() {
    // list_generations enumerates all vars, not just BootOrder
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 200);
    crate::testtoolkit::set_boot_order(&mut store, vec![0]); // 1 not in BootOrder but should be listed
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 2);
}
