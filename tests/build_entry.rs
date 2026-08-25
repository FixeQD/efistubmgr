use crate::DESC_PREFIX;
use crate::boot::build_boot_entry;
use crate::testtoolkit::dummy_hard_drive;
use efivar::boot::BootEntryAttributes;

#[test]
fn build_boot_entry_description() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\Boot\\linux.efi", "cmdline", 123456);
    assert_eq!(e.description, format!("{}{}", DESC_PREFIX, "123456"));
    assert_eq!(e.attributes, BootEntryAttributes::LOAD_OPTION_ACTIVE);
}

#[test]
fn build_boot_entry_loader_path() {
    let hd = dummy_hard_drive(2);
    let e = build_boot_entry(hd.clone(), "\\EFI\\Custom\\boot.efi", "", 0);
    assert_eq!(
        e.file_path_list.as_ref().unwrap().file_path.path,
        "\\EFI\\Custom\\boot.efi"
    );
    assert_eq!(
        e.file_path_list
            .as_ref()
            .unwrap()
            .hard_drive
            .partition_number,
        2
    );
}

#[test]
fn build_boot_entry_optional_data_utf16() {
    let hd = dummy_hard_drive(1);
    let opt = "initrd=\\initramfs.img";
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", opt, 0);
    let decoded: Vec<u16> = e
        .optional_data
        .chunks(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    assert_eq!(decoded, opt.encode_utf16().collect::<Vec<_>>());
}

#[test]
fn build_boot_entry_empty_optional() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "", 0);
    assert!(e.optional_data.is_empty());
}

#[test]
fn build_boot_entry_negative_timestamp() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "", -999);
    assert_eq!(e.description, format!("{}{}", DESC_PREFIX, "-999"));
    assert_eq!(
        crate::timestamp::desc_timestamp(&e.description),
        Some(-999)
    );
}

#[test]
fn build_boot_entry_unicode_optional() {
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