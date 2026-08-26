use crate::nvram::find_free_id;
use crate::testtoolkit::{insert_entry, insert_other_entry};
use efivar::efi::{Variable, VariableFlags};
use efivar::store::MemoryStore;
use efivar::VarWriter;

#[test]
fn find_free_id_empty_store_returns_zero() {
    let store = MemoryStore::new();
    assert_eq!(find_free_id(&store), 0);
}

#[test]
fn find_free_id_first_free_after_sequential() {
    let mut store = MemoryStore::new();
    for i in 0..5 {
        insert_entry(&mut store, i, 100 + i as i64);
    }
    assert_eq!(find_free_id(&store), 5);
}

#[test]
fn find_free_id_skips_holes() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 1);
    insert_entry(&mut store, 2, 2);
    insert_entry(&mut store, 5, 3);
    // 1 is free
    assert_eq!(find_free_id(&store), 1);
    // fill 1, next free is 3
    insert_entry(&mut store, 1, 4);
    assert_eq!(find_free_id(&store), 3);
}

#[test]
fn find_free_id_ignores_non_boot_vars() {
    let mut store = MemoryStore::new();
    store
        .write(
            &Variable::new("BootOrder"),
            VariableFlags::NON_VOLATILE,
            &[0, 0],
        )
        .unwrap();
    store
        .write(
            &Variable::new("SomeOtherVar"),
            VariableFlags::NON_VOLATILE,
            &[1, 2, 3],
        )
        .unwrap();
    // BootOrder and SomeOtherVar shouldn't count as Boot#### slots
    assert_eq!(find_free_id(&store), 0);
}

#[test]
fn find_free_id_with_mixed_desc_and_external() {
    let mut store = MemoryStore::new();
    insert_other_entry(&mut store, 0, "Windows Boot Manager");
    insert_entry(&mut store, 1, 100);
    insert_other_entry(&mut store, 3, "USB Drive");
    assert_eq!(find_free_id(&store), 2);
}

#[test]
fn find_free_id_high_ids() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0xFFF0, 1);
    assert_eq!(find_free_id(&store), 0);
    let mut store2 = MemoryStore::new();
    insert_entry(&mut store2, 0, 1);
    assert_eq!(find_free_id(&store2), 1);
}

#[test]
fn find_free_id_all_low_taken() {
    let mut store = MemoryStore::new();
    for i in 0..10 {
        insert_entry(&mut store, i, i as i64);
    }
    assert_eq!(find_free_id(&store), 10);
}

#[test]
fn find_free_id_case_sensitivity_boot_var() {
    let mut store = MemoryStore::new();
    store
        .write(
            &Variable::new("Boot0000"),
            VariableFlags::NON_VOLATILE,
            &[0],
        )
        .unwrap();
    assert_eq!(find_free_id(&store), 1);
}

#[test]
fn find_free_id_when_only_high_ids_taken() {
    let mut store = MemoryStore::new();
    for id in 0xFF00..=0xFFFF {
        insert_entry(&mut store, id, id as i64);
        if id == 0xFF05 {
            break;
        }
    }
    assert_eq!(find_free_id(&store), 0);
}
