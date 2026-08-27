use crate::boot::build_boot_entry;
use crate::testtoolkit::dummy_hard_drive;
use efivar::boot::BootEntryAttributes;

fn decode_cmdline(data: &[u8]) -> String {
    let split = data.len() - 8;
    let units: Vec<u16> = data[..split - 2]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&units).unwrap()
}

fn decode_ts(data: &[u8]) -> i64 {
    let split = data.len() - 8;
    i64::from_le_bytes(data[split..].try_into().unwrap())
}

#[test]
fn build_boot_entry_description() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\Boot\\linux.efi", "entry", "cmdline", 123456);
    assert_eq!(e.description, "entry");
    assert_eq!(decode_ts(&e.optional_data), 123456);
    assert_eq!(e.attributes, BootEntryAttributes::LOAD_OPTION_ACTIVE);
}

#[test]
fn build_boot_entry_loader_path() {
    let hd = dummy_hard_drive(2);
    let e = build_boot_entry(hd.clone(), "\\EFI\\Custom\\boot.efi", "entry", "", 0);
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
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "entry", opt, 0);
    let decoded = decode_cmdline(&e.optional_data);
    assert_eq!(decoded, opt);
}

#[test]
fn build_boot_entry_empty_optional() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "entry", "", 0);
    // hidden metadata: empty cmdline, then 0x00 0x00 UCS-2 terminator, then 8-byte ts
    assert_eq!(e.optional_data.len(), 10);
    assert_eq!(e.optional_data[0], 0);
    assert_eq!(e.optional_data[1], 0);
    assert_eq!(decode_ts(&e.optional_data), 0);
}

#[test]
fn build_boot_entry_negative_timestamp() {
    let hd = dummy_hard_drive(1);
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "entry", "", -999);
    assert_eq!(e.description, "entry");
    assert_eq!(decode_ts(&e.optional_data), -999);
}

#[test]
fn build_boot_entry_unicode_optional() {
    let hd = dummy_hard_drive(1);
    let opt = "unicode-✓-test";
    let e = build_boot_entry(hd, "\\EFI\\boot.efi", "entry", opt, 1);
    let s = decode_cmdline(&e.optional_data);
    assert_eq!(s, opt);
}
