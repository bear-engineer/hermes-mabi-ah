#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use mabi_ah_app::error::AppError;
use mabi_ah_store::CollectLock;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn setup(content: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let lock = dir.path().join("t.db.collect.lock");
    fs::write(&lock, content).unwrap();
    (dir, db, lock)
}

#[test]
fn lock_with_dead_pid_is_taken_over() {
    let (_d, db, lock) = setup(&format!("999999 {}\n", now()));
    let held = CollectLock::acquire(&db).unwrap();
    let text = fs::read_to_string(&lock).unwrap();
    assert!(text.starts_with(&std::process::id().to_string()), "{text}");
    drop(held);
    assert!(!lock.exists());
}

#[test]
fn lock_older_than_two_hours_is_taken_over() {
    let old = now() - 3 * 3600;
    let (_d, db, _lock) = setup(&format!("{} {old}\n", std::process::id()));
    CollectLock::acquire(&db).unwrap();
}

#[test]
fn lock_held_by_live_process_is_refused() {
    let (_d, db, lock) = setup(&format!("{} {}\n", std::process::id(), now()));
    let err = CollectLock::acquire(&db).unwrap_err();
    assert!(matches!(err, AppError::Store(_)), "{err:?}");
    assert!(lock.exists(), "a live holder's lock must stay");
}

#[test]
fn acquire_writes_pid_and_time() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let _held = CollectLock::acquire(&db).unwrap();
    let text = fs::read_to_string(dir.path().join("t.db.collect.lock")).unwrap();
    let mut it = text.split_whitespace();
    assert_eq!(it.next().unwrap(), std::process::id().to_string());
    assert!(it.next().unwrap().parse::<u64>().unwrap() + 5 >= now());
}
