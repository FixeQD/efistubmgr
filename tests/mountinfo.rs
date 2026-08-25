use crate::mount::{parse_mountinfo_from_str, MountEntry};
use std::path::PathBuf;

#[test]
fn mountinfo_empty() {
    assert_eq!(parse_mountinfo_from_str(""), Vec::<MountEntry>::new());
}

#[test]
fn mountinfo_single_valid_line() {
    let line = "26 23 0:22 / /boot rw,relatime shared:1 - vfat /dev/sda1 rw";
    let v = parse_mountinfo_from_str(line);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].mount_point, PathBuf::from("/boot"));
    assert_eq!(v[0].major, 0);
    assert_eq!(v[0].minor, 22);
}

#[test]
fn mountinfo_multiple_lines() {
    let content = "\
19 23 0:18 / / rw,noatime shared:1 - ext4 /dev/sda2 rw\n\
26 23 0:22 / /boot rw,relatime shared:1 - vfat /dev/sda1 rw\n\
30 23 254:0 / /nix/store ro shared:2 - ext4 /dev/sda3 ro";
    let v = parse_mountinfo_from_str(content);
    assert_eq!(v.len(), 3);
    assert_eq!(v[0].mount_point, PathBuf::from("/"));
    assert_eq!(v[1].mount_point, PathBuf::from("/boot"));
    assert_eq!(v[2].mount_point, PathBuf::from("/nix/store"));
    assert_eq!(v[2].major, 254);
    assert_eq!(v[2].minor, 0);
}

#[test]
fn mountinfo_missing_separator_is_skipped() {
    let content = "26 23 0:22 / /boot rw shared:1 vfat /dev/sda1 rw\n26 23 0:22 / /boot rw shared:1 - vfat /dev/sda1 rw";
    let v = parse_mountinfo_from_str(content);
    assert_eq!(v.len(), 1);
}

#[test]
fn mountinfo_invalid_major_minor_skipped() {
    let line = "26 23 bad:22 / /boot rw - vfat /dev/sda1 rw";
    assert_eq!(parse_mountinfo_from_str(line).len(), 0);
    let line2 = "26 23 0:bad / /boot rw - vfat /dev/sda1 rw";
    assert_eq!(parse_mountinfo_from_str(line2).len(), 0);
    let line3 = "26 23 0-22 / /boot rw - vfat /dev/sda1 rw";
    assert_eq!(parse_mountinfo_from_str(line3).len(), 0);
}

#[test]
fn mountinfo_short_line_skipped() {
    let line = "26 23";
    assert_eq!(parse_mountinfo_from_str(line).len(), 0);
}

#[test]
fn mountinfo_with_spaces_in_mountpoint_escaped() {
    // mountinfo escapes spaces as \040, our simple parser splits on ' ' so it will treat \040 as part of field
    // this tests current behavior: it will still parse but mount_point will be the escaped token
    let line = "26 23 0:22 / /boot\\040with\\040space rw - vfat /dev/sda1 rw";
    let v = parse_mountinfo_from_str(line);
    assert_eq!(v.len(), 1);
    // we don't decode escapes, so raw token is kept
    assert_eq!(v[0].mount_point, PathBuf::from("/boot\\040with\\040space"));
}

#[test]
fn mountinfo_various_majors() {
    let line = "1 1 259:3 / /esp rw - vfat /dev/nvme0n1p1 rw";
    let v = parse_mountinfo_from_str(line);
    assert_eq!(v[0].major, 259);
    assert_eq!(v[0].minor, 3);
}

#[test]
fn mountinfo_blank_lines_ignored() {
    let content = "\n\n26 23 0:22 / /boot rw - vfat /dev/sda1 rw\n\n";
    assert_eq!(parse_mountinfo_from_str(content).len(), 1);
}

#[test]
fn mountinfo_real_world_example() {
    let real = "16 1 0:16 / /sys rw,nosuid,nodev,noexec,relatime shared:7 - sysfs sysfs rw\n\
17 1 0:17 / /proc rw,nosuid,nodev,noexec,relatime shared:8 - proc proc rw\n\
18 1 0:5 / /dev rw,nosuid shared:2 - devtmpfs devtmpfs rw,size=16384k\n\
26 18 259:1 / /boot rw,relatime shared:9 - vfat /dev/nvme0n1p1 rw,fmask=0077";
    let entries = parse_mountinfo_from_str(real);
    assert_eq!(entries.len(), 4);
    let boot = entries
        .iter()
        .find(|e| e.mount_point == PathBuf::from("/boot"))
        .unwrap();
    assert_eq!(boot.major, 259);
    assert_eq!(boot.minor, 1);
}