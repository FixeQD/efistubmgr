use crate::testtoolkit::{dummy_hard_drive, dummy_hard_drive_with_sig};
use efivar::boot::{EFIHardDriveType, FilePath, FilePathList};
use uuid::Uuid;

#[test]
fn hard_drive_roundtrip_gpt() {
    let sig = Uuid::new_v4();
    let hd = dummy_hard_drive_with_sig(2, sig);
    let fp = FilePath {
        path: "\\EFI\\Boot\\linux.efi".to_string(),
    };
    let fpl = FilePathList {
        file_path: fp.clone(),
        hard_drive: hd.clone(),
    };
    let bytes = fpl.to_bytes();
    let opt = FilePathList::parse(&mut &bytes[..]).unwrap();
    let parsed = Option::<FilePathList>::from(opt).unwrap();
    assert_eq!(parsed.hard_drive, hd);
    assert_eq!(parsed.file_path, fp);
}

#[test]
fn hard_drive_roundtrip_different_partitions() {
    for part in [1, 2, 5, 128, u32::MAX] {
        let hd = dummy_hard_drive(part);
        let fpl = FilePathList {
            file_path: FilePath {
                path: "\\EFI\\test.efi".to_string(),
            },
            hard_drive: hd.clone(),
        };
        let bytes = fpl.to_bytes();
        let opt = FilePathList::parse(&mut &bytes[..]).unwrap();
        let parsed = Option::<FilePathList>::from(opt).unwrap();
        assert_eq!(parsed.hard_drive.partition_number, part);
    }
}

#[test]
fn hard_drive_type_is_gpt() {
    let hd = dummy_hard_drive(1);
    assert_eq!(hd.sig_type, EFIHardDriveType::Gpt);
    assert_eq!(hd.format, 0x02);
}

#[test]
fn efi_hard_drive_sig_type_parsing() {
    assert_eq!(EFIHardDriveType::parse(0x01), EFIHardDriveType::Mbr);
    assert_eq!(EFIHardDriveType::parse(0x02), EFIHardDriveType::Gpt);
    assert_eq!(EFIHardDriveType::parse(0xFF), EFIHardDriveType::Unknown);
    assert_eq!(EFIHardDriveType::parse(0x00), EFIHardDriveType::Unknown);
}