use crate::lock::{exclusive_at, open_lock_file, shared_at};

fn temp_lock_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("efistubmgr-test-{name}-{}.lock", std::process::id()))
}

#[test]
fn second_exclusive_lock_on_same_file_blocks() {
    let path = temp_lock_path("exclusive");
    let first = exclusive_at(&path).unwrap();
    let second = open_lock_file(&path).unwrap();
    assert!(second.try_lock().is_err());
    drop(first);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn shared_locks_do_not_conflict_with_each_other() {
    let path = temp_lock_path("shared");
    let first = shared_at(&path).unwrap();
    let second = shared_at(&path).unwrap();
    drop(first);
    drop(second);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn exclusive_lock_blocks_shared_lock() {
    let path = temp_lock_path("mixed");
    let exclusive = exclusive_at(&path).unwrap();
    let shared_attempt = open_lock_file(&path).unwrap();
    assert!(shared_attempt.try_lock_shared().is_err());
    drop(exclusive);
    let _ = std::fs::remove_file(&path);
}
