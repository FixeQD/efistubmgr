use crate::DESC_PREFIX;
use crate::testtoolkit::{
    boot_order, dummy_hard_drive, insert_entry, insert_entry_with_loader, insert_other_entry,
};
use crate::boot::build_boot_entry;
use crate::nvram::list_generations;
use efivar::boot::{BootEntry, BootVarReader, BootVarWriter};
use efivar::efi::{Variable, VariableFlags};
use efivar::store::MemoryStore;
use efivar::{VarReader, VarWriter};

#[test]
fn nvram_persistence_memory_store_roundtrip() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let entry = build_boot_entry(hd.clone(), "\\EFI\\Boot\\boot.efi", "opt", 12345);
    store.add_boot_entry(0x1234, entry.clone()).unwrap();
    store.set_boot_order(vec![0x1234, 0x0001]).unwrap();
    // Simulate reboot: read back
    let read_entry = BootEntry::read(&store, &Variable::new("Boot1234")).unwrap();
    assert_eq!(read_entry.description, format!("{}{}", DESC_PREFIX, "12345"));
    assert_eq!(read_entry.file_path_list.unwrap().hard_drive, hd);
    assert_eq!(store.get_boot_order().unwrap(), vec![0x1234, 0x0001]);
}

#[test]
fn nvram_boot_order_le_encoding() {
    let mut store = MemoryStore::new();
    store.set_boot_order(vec![0x0001, 0x1234, 0xABCD]).unwrap();
    // Verify raw bytes are little endian u16
    let var = Variable::new("BootOrder");
    let (data, _) = store.read(&var).unwrap();
    assert_eq!(data, vec![0x01, 0x00, 0x34, 0x12, 0xCD, 0xAB]);
    // Read back via helper
    assert_eq!(
        store.get_boot_order().unwrap(),
        vec![0x0001, 0x1234, 0xABCD]
    );
}

#[test]
fn nvram_empty_boot_order_bytes() {
    let mut store = MemoryStore::new();
    store.set_boot_order(vec![]).unwrap();
    let (data, _) = store.read(&Variable::new("BootOrder")).unwrap();
    assert!(data.is_empty());
    assert_eq!(store.get_boot_order().unwrap(), Vec::<u16>::new());
}

#[test]
fn nvram_overwrite_boot_order() {
    let mut store = MemoryStore::new();
    store.set_boot_order(vec![1, 2, 3]).unwrap();
    store.set_boot_order(vec![3, 2, 1]).unwrap();
    assert_eq!(store.get_boot_order().unwrap(), vec![3, 2, 1]);
}

#[test]
fn nvram_variable_flags_default() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let entry = build_boot_entry(hd, "\\EFI\\boot.efi", "", 1);
    store.add_boot_entry(0, entry).unwrap();
    let (_, flags) = store.read(&Variable::new("Boot0000")).unwrap();
    // efivar writer uses VariableFlags::default() = NON_VOLATILE | BOOTSERVICE_ACCESS | RUNTIME_ACCESS
    assert!(flags.contains(VariableFlags::NON_VOLATILE));
    assert!(flags.contains(VariableFlags::BOOTSERVICE_ACCESS));
    assert!(flags.contains(VariableFlags::RUNTIME_ACCESS));
}

#[test]
fn nvram_delete_nonexistent_returns_error() {
    let mut store = MemoryStore::new();
    let res = store.delete(&Variable::new("Boot0000"));
    assert!(matches!(res, Err(efivar::Error::VarNotFound { .. })));
}

#[test]
fn nvram_isolation_between_stores() {
    let mut store1 = MemoryStore::new();
    let mut store2 = MemoryStore::new();
    insert_entry(&mut store1, 0, 100);
    insert_entry(&mut store2, 0, 200);
    assert_eq!(list_generations(&store1)[0].ts, 100);
    assert_eq!(list_generations(&store2)[0].ts, 200);
    assert_eq!(boot_order(&store1), vec![]);
}

#[test]
fn nvram_multiple_stores_with_same_id_different_content() {
    let mut s1 = MemoryStore::new();
    let mut s2 = MemoryStore::new();
    insert_entry_with_loader(&mut s1, 0, 100, "\\EFI\\Boot\\a.efi", "a");
    insert_entry_with_loader(&mut s2, 0, 100, "\\EFI\\Boot\\b.efi", "b");
    let e1 = BootEntry::read(&s1, &Variable::new("Boot0000")).unwrap();
    let e2 = BootEntry::read(&s2, &Variable::new("Boot0000")).unwrap();
    assert_ne!(
        e1.file_path_list.unwrap().file_path.path,
        e2.file_path_list.unwrap().file_path.path
    );
}

#[test]
fn nvram_boot_order_with_duplicates() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 200);
    crate::testtoolkit::set_boot_order(&mut store, vec![0, 0, 1, 0]);
    // list_generations should still list unique gens
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 2);
    // simulate_boot will iterate duplicates, but still picks first valid
    assert_eq!(crate::testtoolkit::simulate_boot(&store).unwrap().0, 0);
}

#[test]
fn nvram_many_boot_entries_stress() {
    let mut store = MemoryStore::new();
    for i in 0..100 {
        insert_entry(&mut store, i, i as i64 * 10);
    }
    crate::testtoolkit::set_boot_order(&mut store, (0..100).collect());
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 100);
    assert_eq!(gens[0].ts, 990);
    assert_eq!(gens[99].ts, 0);
    // Simulate boot picks 99 (ts 990)
    assert_eq!(
        crate::testtoolkit::simulate_desc_boot(&store).unwrap(),
        (99, 990)
    );
}

#[test]
fn nvram_all_desc_sorted_even_if_initially_shuffled() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 5, 500);
    insert_entry(&mut store, 2, 200);
    insert_entry(&mut store, 9, 900);
    insert_entry(&mut store, 1, 100);
    crate::testtoolkit::set_boot_order(&mut store, vec![1, 5, 2, 9]); // random
    let hd = dummy_hard_drive(1);
    crate::boot::cmd_create_with_hard_drive(
        &mut store,
        hd,
        "\\EFI\\Boot\\boot.efi",
        "",
        300,
    );
    // Expected sorted: 900,500,300,200,100 -> ids 9,5,new,2,1
    let gens = list_generations(&store);
    let expected_ids: Vec<u16> = gens.iter().map(|g| g.id).collect();
    assert_eq!(boot_order(&store), expected_ids);
    assert_eq!(gens[0].ts, 900);
    assert_eq!(gens[1].ts, 500);
    assert_eq!(gens[2].ts, 300);
}

#[test]
fn efivar_variable_boot_var_id_parsing() {
    assert_eq!(Variable::new("Boot0000").boot_var_id(), Some(0));
    assert_eq!(Variable::new("BootFFFF").boot_var_id(), Some(0xFFFF));
    assert_eq!(Variable::new("Boot000A").boot_var_id(), Some(0xA));
    assert_eq!(Variable::new("BootOrder").boot_var_id(), None);
    assert_eq!(Variable::new("Boot000").boot_var_id(), None);
    assert_eq!(Variable::new("Boot00000").boot_var_id(), None);
    assert_eq!(Variable::new("boot0000").boot_var_id(), None);
    assert_eq!(Variable::new("BootABCD").boot_var_id(), Some(0xABCD));
    assert_eq!(Variable::new("BootG000").boot_var_id(), None);
}

#[test]
fn nvram_variable_vendor_is_efi_by_default() {
    let var = Variable::new("Boot0000");
    assert!(var.vendor().is_efi());
    assert_eq!(
        var.to_string(),
        "Boot0000-8be4df61-93ca-11d2-aa0d-00e098032b8c"
    );
}

#[test]
fn boot_entries_iter_via_get_boot_entries() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 200);
    crate::testtoolkit::set_boot_order(&mut store, vec![0, 1]);
    let iter = store.get_boot_entries().unwrap();
    let collected: Vec<_> = iter.collect();
    assert_eq!(collected.len(), 2);
    let ids: Vec<u16> = collected
        .iter()
        .map(|(res, _var)| res.as_ref().unwrap().id)
        .collect();
    assert!(ids.contains(&0) && ids.contains(&1));
}

#[test]
fn nvram_list_generations_ignores_other_filtered() {
    let mut store = MemoryStore::new();
    insert_other_entry(&mut store, 0, "Windows");
    insert_other_entry(&mut store, 1, "ubuntu");
    // Only entry counted
    let gens = list_generations(&store);
    assert!(gens.is_empty());
    insert_entry(&mut store, 2, 100);
    assert_eq!(list_generations(&store).len(), 1);
}