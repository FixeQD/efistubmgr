use std::path::{Path, PathBuf};

use efivar::boot::{EFIHardDrive, EFIHardDriveType};
use eros::Context;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MountEntry {
    pub mount_point: PathBuf,
    pub major: u32,
    pub minor: u32,
}

/// Parse mountinfo content into entries.
pub fn parse_mountinfo_from_str(content: &str) -> Vec<MountEntry> {
    content
        .lines()
        .filter_map(|line| {
            let (pre, _post) = line.split_once(" - ")?;
            let fields: Vec<&str> = pre.split(' ').collect();
            let majmin = fields.get(2)?;
            let (major, minor) = majmin.split_once(':')?;
            Some(MountEntry {
                mount_point: PathBuf::from(*fields.get(4)?),
                major: major.parse().ok()?,
                minor: minor.parse().ok()?,
            })
        })
        .collect()
}

pub fn try_parse_mountinfo() -> eros::Result<Vec<MountEntry>> {
    let content =
        std::fs::read_to_string("/proc/self/mountinfo").context("reading /proc/self/mountinfo")?;
    Ok(parse_mountinfo_from_str(&content))
}

pub fn try_resolve_partition(mount_point: &Path) -> eros::Result<(String, u32)> {
    let canon = std::fs::canonicalize(mount_point)
        .with_context(|| format!("resolving {}", mount_point.display()))?;

    let entry = try_parse_mountinfo()?
        .into_iter()
        .rfind(|e| {
            std::fs::canonicalize(&e.mount_point)
                .map(|p| p == canon)
                .unwrap_or(false)
        })
        .ok_or_else(|| eros::error!("no mount found for {}", mount_point.display()))?;

    let sys_link = format!("/sys/dev/block/{}:{}", entry.major, entry.minor);
    let target = std::fs::read_link(&sys_link).with_context(|| format!("reading {sys_link}"))?;

    let part_name = target
        .file_name()
        .ok_or_else(|| eros::error!("malformed sysfs block symlink at {sys_link}"))?
        .to_string_lossy()
        .into_owned();

    let disk_name = target
        .parent()
        .and_then(|p| p.file_name())
        .ok_or_else(|| eros::error!("malformed sysfs block symlink at {sys_link} (no parent)"))?
        .to_string_lossy()
        .into_owned();

    let partition_number: u32 =
        std::fs::read_to_string(format!("/sys/class/block/{part_name}/partition"))
            .with_context(|| format!("reading partition number for {part_name}"))?
            .trim()
            .parse::<u32>()
            .with_context(|| format!("parsing partition number for {part_name}"))?;

    Ok((disk_name, partition_number))
}

pub fn try_read_logical_block_size(disk_name: &str) -> eros::Result<gpt::disk::LogicalBlockSize> {
    let raw: u64 =
        std::fs::read_to_string(format!("/sys/block/{disk_name}/queue/logical_block_size"))
            .with_context(|| format!("reading logical_block_size for {disk_name}"))?
            .trim()
            .parse::<u64>()
            .with_context(|| format!("parsing logical_block_size for {disk_name}"))?;

    raw.try_into()
        .map_err(|_| eros::error!("unsupported logical block size {raw} on {disk_name}"))
}

pub fn try_build_hard_drive(esp_mount_point: &Path) -> eros::Result<EFIHardDrive> {
    let (disk_name, partition_number) = try_resolve_partition(esp_mount_point)?;
    let lb_size = try_read_logical_block_size(&disk_name)?;

    let disk_path = format!("/dev/{disk_name}");
    let disk = gpt::GptConfig::new()
        .writable(false)
        .logical_block_size(lb_size)
        .open(&disk_path)
        .with_context(|| format!("reading GPT table on {disk_path}"))?;

    let part = disk.partitions().get(&partition_number).ok_or_else(|| {
        eros::error!("partition {partition_number} not found in GPT table on {disk_path}")
    })?;

    Ok(EFIHardDrive {
        partition_number,
        partition_start: part.first_lba,
        partition_size: part.last_lba - part.first_lba + 1,
        partition_sig: part.part_guid,
        format: 0x02,
        sig_type: EFIHardDriveType::Gpt,
    })
}
