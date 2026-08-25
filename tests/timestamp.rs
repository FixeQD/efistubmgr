use crate::timestamp::desc_timestamp;
use crate::DESC_PREFIX;

#[test]
fn timestamp_simple_positive() {
    assert_eq!(desc_timestamp(&format!("{}123", DESC_PREFIX)), Some(123));
}

#[test]
fn timestamp_negative() {
    assert_eq!(desc_timestamp(&format!("{}-42", DESC_PREFIX)), Some(-42));
}

#[test]
fn timestamp_zero() {
    assert_eq!(desc_timestamp(&format!("{}0", DESC_PREFIX)), Some(0));
}

#[test]
fn timestamp_with_leading_trim() {
    assert_eq!(desc_timestamp(&format!("{}  999", DESC_PREFIX)), Some(999));
    assert_eq!(desc_timestamp(&format!("{}\t100", DESC_PREFIX)), Some(100));
    assert_eq!(desc_timestamp(&format!("{}  0  ", DESC_PREFIX)), Some(0));
}

#[test]
fn timestamp_trailing_spaces() {
    assert_eq!(desc_timestamp(&format!("{}123   ", DESC_PREFIX)), Some(123));
}

#[test]
fn timestamp_large_i64_max() {
    let max = i64::MAX.to_string();
    assert_eq!(desc_timestamp(&format!("{}{}", DESC_PREFIX, max)), Some(i64::MAX));
}

#[test]
fn timestamp_large_i64_min() {
    let min = i64::MIN.to_string();
    assert_eq!(desc_timestamp(&format!("{}{}", DESC_PREFIX, min)), Some(i64::MIN));
}

#[test]
fn timestamp_overflow_returns_none() {
    // i64 overflow - parse fails -> warning + None
    assert_eq!(desc_timestamp(&format!("{}9999999999999999999999", DESC_PREFIX)), None);
}

#[test]
fn timestamp_not_prefixed() {
    assert_eq!(desc_timestamp(&format!("{}x123", DESC_PREFIX)), None);
    assert_eq!(desc_timestamp("efistub123"), None);
    assert_eq!(desc_timestamp(" Entry123"), None);
    assert_eq!(desc_timestamp(""), None);
    assert_eq!(desc_timestamp("Other123"), None);
    assert_eq!(desc_timestamp("Boot123"), None);
}

#[test]
fn timestamp_only_prefix_no_number() {
    assert_eq!(desc_timestamp(DESC_PREFIX), None);
    assert_eq!(desc_timestamp(&format!("{}   ", DESC_PREFIX)), None);
    assert_eq!(desc_timestamp(&format!("{}\t", DESC_PREFIX)), None);
}

#[test]
fn timestamp_unparsable_suffix() {
    assert_eq!(desc_timestamp(&format!("{}abc", DESC_PREFIX)), None);
    assert_eq!(desc_timestamp(&format!("{}12abc", DESC_PREFIX)), None);
    assert_eq!(desc_timestamp(&format!("{}12.34", DESC_PREFIX)), None);
    assert_eq!(desc_timestamp(&format!("{}--5", DESC_PREFIX)), None);
    assert_eq!(desc_timestamp(&format!("{}++5", DESC_PREFIX)), None);
}

#[test]
fn timestamp_float_string_is_none() {
    assert_eq!(desc_timestamp(&format!("{}3.14", DESC_PREFIX)), None);
}

#[test]
fn timestamp_hex_not_parsed_as_decimal() {
    assert_eq!(desc_timestamp(&format!("{}0x10", DESC_PREFIX)), None);
}

#[test]
fn timestamp_unicode_trim() {
    // only ASCII trim is relevant; but test that unicode prefix fails
    assert_eq!(desc_timestamp(&format!("{}１２３", DESC_PREFIX)), None);
}

#[test]
fn timestamp_embedded_newline() {
    assert_eq!(desc_timestamp(&format!("{}123\n", DESC_PREFIX)), Some(123)); // trim removes newline
    assert_eq!(
        desc_timestamp(format!("{}{}", DESC_PREFIX, "123\n").trim_end_matches('\n')),
        Some(123)
    );
}

#[test]
fn timestamp_preserves_sign() {
    assert_eq!(desc_timestamp(&format!("{}+5", DESC_PREFIX)), Some(5));
    assert_eq!(desc_timestamp(&format!("{}-0", DESC_PREFIX)), Some(0));
}

#[test]
fn timestamp_empty_after_trim() {
    assert_eq!(desc_timestamp(&format!("{} ", DESC_PREFIX)), None);
}

#[test]
fn timestamp_with_internal_spaces() {
    assert_eq!(desc_timestamp(&format!("{}1 2", DESC_PREFIX)), None);
    assert_eq!(desc_timestamp(&format!("{} 1 2", DESC_PREFIX)), None);
}
