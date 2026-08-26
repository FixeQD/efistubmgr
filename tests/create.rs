use crate::boot::cmd_create_with_hard_drive;
use crate::nvram::list_generations;
use crate::testtoolkit::{
    boot_order, dummy_hard_drive, insert_entry, insert_other_entry, set_boot_order,
};
use efivar::boot::{BootEntry, BootVarReader};
use efivar::efi::Variable;
use efivar::store::MemoryStore;

/// Decode hidden cmdline from optional_data
fn decode_cmdline(data: &[u8]) -> String {
    let split = data.len() - 8;
    String::from_utf8_lossy(&data[..split - 1]).to_string()
}

#[test]
fn create_single_entry_sets_boot_order() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let id = cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 1000);
    assert_eq!(id, 0);
    assert_eq!(boot_order(&store), vec![0]);
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 1);
    assert_eq!(gens[0].ts, 1000);
}

#[test]
fn create_entry_sorts_boot_order_newest_first() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        100,
    );
    cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        300,
    );
    cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        200,
    );
    // After each create, BootOrder is re-synced to sorted Entry ids
    // Final order should be ts 300,200,100 -> ids 1,2,0
    let order = boot_order(&store);
    assert_eq!(order.len(), 3);
    // Verify generations sorted
    let gens = list_generations(&store);
    assert_eq!(gens[0].ts, 300);
    assert_eq!(gens[1].ts, 200);
    assert_eq!(gens[2].ts, 100);
    // Order must match gens order
    assert_eq!(order, gens.iter().map(|g| g.id).collect::<Vec<_>>());
}

#[test]
fn create_preserves_other_entries_at_end() {
    let mut store = MemoryStore::new();
    insert_other_entry(&mut store, 10, "Windows Boot Manager");
    insert_other_entry(&mut store, 11, "USB");
    set_boot_order(&mut store, vec![10, 11]);
    let hd = dummy_hard_drive(1);
    cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        500,
    );
    cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 100);
    let order = boot_order(&store);
    // Entry sorted newest first, then non-Entry preserved
    // ids: first create uses id 0, second uses id 1 (since 10,11 occupied but 0 free)
    let gens = list_generations(&store);
    assert_eq!(gens[0].id, 0);
    assert_eq!(gens[1].id, 1);
    assert_eq!(order, vec![0, 1, 10, 11]);
}

#[test]
fn create_with_existing_entry_re_sorts() {
    let mut store = MemoryStore::new();
    // Pre-create 2 entry via direct insert but without BootOrder sync (like leftover from old tool)
    insert_entry(&mut store, 5, 100);
    insert_entry(&mut store, 2, 300);
    set_boot_order(&mut store, vec![5, 2]); // wrong order (oldest first)
    let hd = dummy_hard_drive(1);
    let new_id =
        cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 200);
    // Now BootOrder should be sorted 300,200,100 -> ids 2, new_id,5
    let order = boot_order(&store);
    let gens = list_generations(&store);
    assert_eq!(
        gens.iter().map(|g| g.ts).collect::<Vec<_>>(),
        vec![300, 200, 100]
    );
    assert_eq!(order, gens.iter().map(|g| g.id).collect::<Vec<_>>());
    assert!(order.contains(&new_id));
}

#[test]
fn create_handles_missing_boot_order() {
    let mut store = MemoryStore::new();
    // No BootOrder variable yet
    assert!(store.get_boot_order().is_err());
    let hd = dummy_hard_drive(1);
    cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 42);
    assert_eq!(boot_order(&store), vec![0]);
}

#[test]
fn create_boot_order_empty_initially() {
    let mut store = MemoryStore::new();
    set_boot_order(&mut store, vec![]);
    let hd = dummy_hard_drive(1);
    cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 1);
    assert_eq!(boot_order(&store), vec![0]);
}

#[test]
fn create_multiple_entry_and_other_interleaved() {
    let mut store = MemoryStore::new();
    // Simulate existing state: BootOrder = [0x0001 (Entry 100), 0x000A (Windows), 0x0002 (Entry 200)]
    insert_entry(&mut store, 1, 100);
    insert_entry(&mut store, 2, 200);
    insert_other_entry(&mut store, 0xA, "Windows");
    set_boot_order(&mut store, vec![1, 0xA, 2]);
    let hd = dummy_hard_drive(1);
    cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 300);
    // new entry id 0 (free), sorted should be 300,200,100 -> ids 0,2,1 then Windows 0xA
    let order = boot_order(&store);
    assert_eq!(order, vec![0, 2, 1, 0xA]);
}

#[test]
fn create_find_free_id_uses_hole() {
    let mut store = MemoryStore::new();
    insert_entry(&mut store, 0, 100);
    insert_entry(&mut store, 2, 200);
    // hole at 1
    let hd = dummy_hard_drive(1);
    let id = cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 150);
    assert_eq!(id, 1);
}

#[test]
fn create_preserves_optional_data() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let opt = "root=PARTUUID=123 quiet";
    cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", opt, 123);
    let entry = BootEntry::read(&store, &Variable::new("Boot0000")).unwrap();
    let decoded = decode_cmdline(&entry.optional_data);
    assert_eq!(decoded, opt);
}

#[test]
fn create_verifies_file_path_persisted() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(5);
    let loader = "\\EFI\\Boot\\nixos-abc.efi";
    cmd_create_with_hard_drive(&mut store, hd.clone(), loader, "entry", "", 999);
    let entry = BootEntry::read(&store, &Variable::new("Boot0000")).unwrap();
    let fpl = entry.file_path_list.unwrap();
    assert_eq!(fpl.file_path.path, loader);
    assert_eq!(fpl.hard_drive.partition_number, 5);
    assert_eq!(fpl.hard_drive.partition_sig, hd.partition_sig);
}

#[test]
fn create_with_duplicate_timestamp() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let id1 = cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        1000,
    );
    let id2 = cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        1000,
    );
    assert_ne!(id1, id2);
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 2);
    assert_eq!(gens[0].ts, 1000);
    assert_eq!(gens[1].ts, 1000);
}

#[test]
fn create_large_number_of_generations() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    for i in 0..50 {
        cmd_create_with_hard_drive(
            &mut store,
            hd.clone(),
            "\\EFI\\Boot\\boot.efi",
            "entry",
            "",
            i,
        );
    }
    let gens = list_generations(&store);
    assert_eq!(gens.len(), 50);
    // newest first
    assert_eq!(gens[0].ts, 49);
    assert_eq!(gens[49].ts, 0);
    // BootOrder should have 50 entries, sorted descending
    let order = boot_order(&store);
    assert_eq!(order.len(), 50);
    assert_eq!(order, gens.iter().map(|g| g.id).collect::<Vec<_>>());
}

#[test]
fn create_with_different_partition_numbers() {
    let mut store = MemoryStore::new();
    for part in [1, 2, 5, 10] {
        let hd = dummy_hard_drive(part);
        let loader = format!("\\EFI\\Boot\\boot{part}.efi");
        let id =
            cmd_create_with_hard_drive(&mut store, hd, &loader, "entry", "", part as i64 * 100);
        let entry = BootEntry::read(&store, &Variable::new(&format!("Boot{id:04X}"))).unwrap();
        assert_eq!(
            entry.file_path_list.unwrap().hard_drive.partition_number,
            part
        );
    }
}

#[test]
fn create_with_unicode_loader_and_data() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let loader = "\\EFI\\Boot\\linux-über.efi";
    let data = "console=ttyS0,115200 ✓";
    let id = cmd_create_with_hard_drive(&mut store, hd, loader, "entry", data, 42);
    let entry = BootEntry::read(&store, &Variable::new(&format!("Boot{id:04X}"))).unwrap();
    assert_eq!(entry.file_path_list.unwrap().file_path.path, loader);
    let decoded = decode_cmdline(&entry.optional_data);
    assert_eq!(decoded, data);
}

#[test]
fn create_preserves_existing_other_boot_order_on_empty_entry() {
    let mut store = MemoryStore::new();
    insert_other_entry(&mut store, 10, "Windows");
    set_boot_order(&mut store, vec![10]);
    let hd = dummy_hard_drive(1);
    let new_id =
        cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 1);
    assert_eq!(boot_order(&store), vec![new_id, 10]);
}

#[test]
fn create_does_not_duplicate_boot_order_entries() {
    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let id0 = cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        100,
    );
    let _id1 =
        cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 200);
    let order2 = boot_order(&store);
    assert_eq!(order2.len(), 2);
    // No duplicate ids
    let unique: std::collections::HashSet<_> = order2.iter().collect();
    assert_eq!(unique.len(), order2.len());
    assert!(order2.contains(&id0));
}

#[test]
fn create_boot_order_retains_other_relative_order() {
    let mut store = MemoryStore::new();
    insert_other_entry(&mut store, 10, "Windows");
    insert_other_entry(&mut store, 11, "Ubuntu");
    insert_other_entry(&mut store, 12, "PXE");
    set_boot_order(&mut store, vec![10, 11, 12]);
    let hd = dummy_hard_drive(1);
    cmd_create_with_hard_drive(
        &mut store,
        hd.clone(),
        "\\EFI\\Boot\\boot.efi",
        "entry",
        "",
        100,
    );
    cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\Boot\\boot.efi", "entry", "", 200);
    let order = boot_order(&store);
    // Entry at front sorted, then non-entry in original order
    assert_eq!(order, vec![1, 0, 10, 11, 12]);
}
