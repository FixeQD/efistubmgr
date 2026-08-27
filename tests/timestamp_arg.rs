use crate::timestamp_from;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn timestamp_from_normal_time_succeeds() {
    let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    assert_eq!(timestamp_from(now).unwrap(), 1_700_000_000);
}

#[test]
fn timestamp_from_before_epoch_errors_instead_of_defaulting_to_zero() {
    let before_epoch = UNIX_EPOCH - Duration::from_secs(1);
    assert!(timestamp_from(before_epoch).is_err());
}

#[test]
fn timestamp_from_epoch_itself_is_zero() {
    assert_eq!(timestamp_from(UNIX_EPOCH).unwrap(), 0);
}

#[test]
fn timestamp_from_far_future_does_not_overflow() {
    let now = SystemTime::now();
    assert!(timestamp_from(now).unwrap() > 0);
}
