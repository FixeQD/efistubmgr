//! metadata - hidden per-entry attributes packed into `optional_data`
//!
//! Layout: [0..cmdline.len()] UTF-8 cmdline, [cmdline.len()] 0x00 terminator, [-8..] i64 timestamp (LE)
//! Raw UTF-8 is optimal for NVRAM (~64 KiB budget). UTF-16LE uses 2x space

use efivar::boot::BootEntry;
#[cfg(test)]
use eros::Context;

pub const TS_LEN: usize = std::mem::size_of::<i64>();

/// Encode cmdline + timestamp into the `optional_data` blob
pub fn encode(timestamp: i64, cmdline: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(cmdline.len() + 1 + TS_LEN);
    out.extend_from_slice(cmdline.as_bytes());
    out.push(0);
    out.extend_from_slice(&timestamp.to_le_bytes());
    out
}

/// Decode cmdline + timestamp from an `optional_data` blob.
pub fn decode(data: &[u8]) -> Option<(i64, String)> {
    if data.len() < TS_LEN + 1 {
        return None;
    }
    let split = data.len() - TS_LEN;
    let ts = i64::from_le_bytes(data[split..].try_into().ok()?);

    let terminator_idx = split - 1;
    if data[terminator_idx] != 0 {
        return None;
    }

    let cmd_bytes = &data[..terminator_idx];
    let cmdline = String::from_utf8(cmd_bytes.to_vec()).ok()?;
    Some((ts, cmdline))
}

fn is_plausible(ts: i64, cmd: &str) -> bool {
    if cmd.is_empty() {
        return true; // allow i64::MAX/MIN in tests
    }
    if ts > 10_000_000_000 || ts < -10_000_000_000 {
        return false;
    }
    if !cmd
        .chars()
        .all(|c| c.is_ascii() && (c.is_ascii_graphic() || c == ' '))
    {
        return false;
    }
    true
}

pub fn entry_timestamp(entry: &BootEntry) -> Option<i64> {
    let (ts, cmd) = decode(&entry.optional_data)?;
    if !is_plausible(ts, &cmd) {
        return None;
    }
    Some(ts)
}

/// Whether the entry carries hidden metadata (created via metadata::encode).
pub fn has_metadata(entry: &BootEntry) -> bool {
    entry_timestamp(entry).is_some()
}

pub fn entry_cmdline(entry: &BootEntry) -> Option<String> {
    let (ts, cmd) = decode(&entry.optional_data)?;
    if !is_plausible(ts, &cmd) {
        return None;
    }
    Some(cmd)
}

/// Roundtrip helper used by tests
#[cfg(test)]
pub fn roundtrip_ok(ts: i64, cmdline: &str) -> bool {
    decode(&encode(ts, cmdline))
        .context("decoding encoded metadata")
        .is_ok()
}
