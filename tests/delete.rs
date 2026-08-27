use crate::boot::cmd_create_with_hard_drive;
use crate::cmd_delete;
use crate::nvram::list_generations;
use crate::testtoolkit::{boot_order, insert_entry, insert_other_entry, set_boot_order};
use efivar::VarReader;
use efivar::boot::BootVarReader;
use efivar::efi::Variable;
use efivar::store::MemoryStore;

#[test]
fn delete_removes_variable_and_boot_order() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 200);
    set_boot_order(&mut store, vec![1, 0]);
    cmd_delete(&mut store, 1);
    assert!(store.read(&Variable::new("Boot0001")).is_err());
    assert_eq!(boot_order(&store), vec![0]);
    assert!(store.read(&Variable::new("Boot0000")).is_ok());
}

#[test]
fn delete_nonexistent_in_boot_order_leaves_order() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    set_boot_order(&mut store, vec![0, 5]); // 5 doesn't exist as variable but in order (maybe USB)
    cmd_delete(&mut store, 0);
    // BootOrder should have 5 still, 0 removed
    assert_eq!(boot_order(&store), vec![5]);
}

#[test]
fn delete_id_not_in_boot_order_still_deletes_var() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 200);
    set_boot_order(&mut store, vec![0]); // 1 not in order
    cmd_delete(&mut store, 1);
    assert!(store.read(&Variable::new("Boot0001")).is_err());
    assert_eq!(boot_order(&store), vec![0]); // unchanged
}

#[test]
fn delete_missing_boot_order_is_ok() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    // No BootOrder variable
    assert!(store.get_boot_order().is_err());
    cmd_delete(&mut store, 0);
    assert!(store.read(&Variable::new("Boot0000")).is_err());
    // After delete with missing BootOrder, we don't create BootOrder (code returns early because before==len check)
    assert!(store.get_boot_order().is_err());
}

#[test]
fn delete_removes_all_occurrences_in_boot_order() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    set_boot_order(&mut store, vec![0, 1, 0, 2, 0]);
    cmd_delete(&mut store, 0);
    assert_eq!(boot_order(&store), vec![1, 2]);
}

#[test]
fn delete_preserves_other_order() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_other_entry(&mut store, 10, "Windows");
    insert_other_entry(&mut store, 11, "USB");
    set_boot_order(&mut store, vec![0, 10, 11]);
    cmd_delete(&mut store, 0);
    assert_eq!(boot_order(&store), vec![10, 11]);
}

#[test]
fn delete_then_create_reuses_id() {
    let mut store = MemoryStore::new();
    let hd = crate::testtoolkit::dummy_hard_drive(1);
    let id0 = cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        100,
    );
    assert_eq!(id0, 0);
    cmd_delete(&mut store, 0);
    let id1 = cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 200);
    assert_eq!(id1, 0); // reused
}

#[test]
fn delete_then_list_generations() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 300);
    insert_entry(&mut store, 1, 100);
    insert_entry(&mut store, 2, 200);
    set_boot_order(&mut store, vec![0, 2, 1]);
    cmd_delete(&mut store, 0);
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 2);
    assert_eq!(gens[0].ts, 200);
    assert_eq!(gens[1].ts, 100);
    // BootOrder still contains id for deleted? we deleted 0, order should be [2,1]
    assert_eq!(boot_order(&store), vec![2, 1]);
}

#[test]
fn delete_boot_order_with_duplicates() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 1, 200);
    set_boot_order(&mut store, vec![0, 0, 1, 0]);
    cmd_delete(&mut store, 0);
    assert_eq!(boot_order(&store), vec![1]);
}
