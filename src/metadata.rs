//! Hidden per-entry attributes packed into `optional_data`, the real UEFI
//! LoadOptions the Linux stub reads as UCS-2 up to the first 0x0000
//! Layout: [UTF-16LE cmdline][0x00 0x00][i64 timestamp, LE], ts at a fixed soffset from the end so it's never confused with a zero mid-string.

use efivar::boot::BootEntry;
#[cfg(test)]
use eros::Context;

pub const TS_LEN: usize = std::mem::size_of::<i64>();
const TERMINATOR_LEN: usize = 2; // one UCS-2 NUL code unit

/// Encode cmdline + timestamp into the `optional_data` blob
pub fn encode(timestamp: i64, cmdline: &str) -> Vec<u8> {
    let units: Vec<u16> = cmdline.encode_utf16().collect();
    let mut out = Vec::with_capacity(units.len() * 2 + TERMINATOR_LEN + TS_LEN);
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out.extend_from_slice(&[0x00, 0x00]); // real UCS-2 NUL, stub stops here
    out.extend_from_slice(&timestamp.to_le_bytes());
    out
}

/// Decode cmdline + timestamp from an `optional_data` blob.
pub fn decode(data: &[u8]) -> Option<(i64, String)> {
    if data.len() < TERMINATOR_LEN + TS_LEN {
        return None;
    }
    let split = data.len() - TS_LEN;
    let ts = i64::from_le_bytes(data[split..].try_into().ok()?);

    let terminator_idx = split - TERMINATOR_LEN;
    if terminator_idx % 2 != 0 {
        return None; // cmdline must be a whole number of UTF-16 code units
    }
    if data[terminator_idx] != 0 || data[terminator_idx + 1] != 0 {
        return None;
    }

    let units: Vec<u16> = data[..terminator_idx]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let cmdline = String::from_utf16(&units).ok()?;
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
