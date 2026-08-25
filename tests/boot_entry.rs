use crate::DESC_PREFIX;
use crate::boot::build_boot_entry;
use crate::testtoolkit::dummy_hard_drive;
use efivar::boot::{BootEntry, BootEntryAttributes, FilePath, FilePathList};

#[test]
fn boot_entry_roundtrip_basic() {
    let hd = dummy_hard_drive(1);
    let entry = BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: format!("{}{}", DESC_PREFIX, "123456").to_string(),
        file_path_list: Some(FilePathList {
            file_path: FilePath {
                path: "\\EFI\\Boot\\boot.efi".to_string(),
            },
            hard_drive: hd.clone(),
        }),
        optional_data: vec![1, 2, 3, 4],
    };
    let bytes = entry.to_bytes();
    let parsed = BootEntry::parse(bytes).unwrap();
    assert_eq!(parsed, entry);
}

#[test]
fn boot_entry_roundtrip_with_optional_utf16() {
    let hd = dummy_hard_drive(1);
    let optional = "root=UUID=test quiet";
    let entry = build_boot_entry(hd.clone(), "\\EFI\\Boot\\boot.efi", optional, 999);
    let bytes = entry.to_bytes();
    let parsed = BootEntry::parse(bytes).unwrap();
    assert_eq!(parsed.description, format!("{}{}", DESC_PREFIX, "999"));
    assert_eq!(
        parsed.file_path_list.unwrap().file_path.path,
        "\\EFI\\Boot\\boot.efi"
    );
    let expected: Vec<u16> = optional.encode_utf16().collect();
    let decoded_u16: Vec<u16> = parsed
        .optional_data
        .chunks(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    assert_eq!(decoded_u16, expected);
    let decoded_str = String::from_utf16(&decoded_u16).unwrap();
    assert_eq!(decoded_str, optional);
}

#[test]
fn boot_entry_roundtrip_no_file_path_list() {
    let entry = BootEntry {
        attributes: BootEntryAttributes::LOAD_OPTION_ACTIVE,
        description: format!("{}{}", DESC_PREFIX, "123").to_string(),
        file_path_list: None,
        optional_data: vec![],
    };
    let bytes = entry.to_bytes();
    let parsed = BootEntry::parse(bytes).unwrap();
    assert_eq!(parsed.file_path_list, None);
    assert_eq!(parsed.description, format!("{}{}", DESC_PREFIX, "123"));
}

#[test]
fn boot_entry_attributes_preserved() {
    let hd = dummy_hard_drive(1);
    let attrs = BootEntryAttributes::LOAD_OPTION_ACTIVE | BootEntryAttributes::LOAD_OPTION_HIDDEN;
    let entry = BootEntry {
        attributes: attrs,
        description: format!("{}{}", DESC_PREFIX, "1").to_string(),
        file_path_list: Some(FilePathList {
            file_path: FilePath {
                path: "\\EFI\\a.efi".into(),
            },
            hard_drive: hd,
        }),
        optional_data: vec![],
    };
    let parsed = BootEntry::parse(entry.to_bytes()).unwrap();
    assert_eq!(parsed.attributes, attrs);
    assert!(parsed
        .attributes
        .contains(BootEntryAttributes::LOAD_OPTION_HIDDEN));
}

#[test]
fn boot_entry_unicode_loader_path() {
    let hd = dummy_hard_drive(1);
    let path = "\\EFI\\Boot\\über.efi";
    let entry = build_boot_entry(hd, path, "", 1);
    let parsed = BootEntry::parse(entry.to_bytes()).unwrap();
    assert_eq!(parsed.file_path_list.unwrap().file_path.path, path);
}

#[test]
fn boot_entry_large_optional() {
    let hd = dummy_hard_drive(1);
    let large_opt = "a".repeat(1000);
    let entry = build_boot_entry(hd, "\\EFI\\boot.efi", &large_opt, 123);
    let parsed = BootEntry::parse(entry.to_bytes()).unwrap();
    let decoded = String::from_utf16(
        &parsed
            .optional_data
            .chunks(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<u16>>(),
    )
    .unwrap();
    assert_eq!(decoded, large_opt);
}

#[test]
fn boot_entry_unicode_optional() {
    let hd = dummy_hard_drive(1);
    let opt = "unicode-✓-test";
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", opt, 1);
    let decoded: Vec<u16> = e
        .optional_data
        .chunks(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let s = String::from_utf16(&decoded).unwrap();
    assert_eq!(s, opt);
}

#[test]
fn boot_entry_description_with_max_timestamp() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "", i64::MAX);
    assert_eq!(
        crate::timestamp::desc_timestamp(&e.description),
        Some(i64::MAX)
    );
    let parsed = BootEntry::parse(e.to_bytes()).unwrap();
    assert_eq!(parsed.description, format!("{}{}", DESC_PREFIX, i64::MAX));
}

#[test]
fn boot_entry_description_with_min_timestamp() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "", i64::MIN);
    assert_eq!(
        crate::timestamp::desc_timestamp(&e.description),
        Some(i64::MIN)
    );
}

#[test]
fn optional_data_preserves_utf16_with_nulls() {
    use efivar::efi::Variable;
    use efivar::store::MemoryStore;

    let mut store = MemoryStore::new();
    let hd = dummy_hard_drive(1);
    let opt = "test\0with\0nulls";
    let id =
        crate::boot::cmd_create_with_hard_drive(&mut store, hd, "\\EFI\\boot.efi", opt, 1);
    let entry = BootEntry::read(&store, &Variable::new(&format!("Boot{id:04X}"))).unwrap();
    let decoded: Vec<u16> = entry
        .optional_data
        .chunks(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let s = String::from_utf16(&decoded).unwrap();
    assert_eq!(s, opt);
}