use catch_core::{todos, tracking};
use catch_storage::{Disk, FileDisk, ProfileLock, TodosStore, TrackingStore};
use std::{
    collections::BTreeMap,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Memory {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
    failure: Mutex<Option<(String, bool, bool)>>,
}
impl Memory {
    fn fail(&self, name: &str, after: bool, crash: bool) {
        *self.failure.lock().unwrap() = Some((name.into(), after, crash));
    }
    fn fault(&self, name: &str, after: bool) -> Result<(), String> {
        let crash = {
            let mut failure = self.failure.lock().unwrap();
            if failure
                .as_ref()
                .is_some_and(|value| value.0 == name && value.1 == after)
            {
                failure.take().map(|value| value.2)
            } else {
                None
            }
        };
        if let Some(crash) = crash {
            if crash {
                panic!("simulated process termination at {name}");
            }
            return Err("disk full".into());
        }
        Ok(())
    }
    fn put(&self, name: &str, value: &[u8]) {
        self.files
            .lock()
            .unwrap()
            .insert(name.into(), value.to_vec());
    }
}
impl Disk for Memory {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(self.files.lock().unwrap().get(name).cloned())
    }
    fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.fault(name, false)?;
        self.put(name, bytes);
        self.fault(name, true)
    }
    fn remove(&self, name: &str) -> Result<(), String> {
        self.fault(name, false)?;
        self.files.lock().unwrap().remove(name);
        self.fault(name, true)
    }
}
fn issue(
    store: &mut TrackingStore,
    command: tracking::Command,
    now: i64,
    id: &str,
) -> Result<tracking::Document, String> {
    store.command(
        &command,
        store.snapshot().revision,
        tracking::Context { now, new_id: id },
    )
}
fn start(store: &mut TrackingStore) {
    issue(
        store,
        tracking::Command::Start {
            name: "阅读".into(),
            todo_id: Some("todo-1".into()),
        },
        0,
        "a",
    )
    .unwrap();
}

#[test]
fn failed_save_keeps_committed_state_and_does_not_admit_a_duplicate() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk.clone(), 0, "open-1").unwrap();
    start(&mut store);
    let before = store.snapshot().clone();
    let bytes = disk.read("activity-log.json").unwrap();
    disk.fail("activity-log.json", false, false);
    assert!(issue(
        &mut store,
        tracking::Command::Start {
            name: "B".into(),
            todo_id: None
        },
        1000,
        "b"
    )
    .is_err());
    assert_eq!(*store.snapshot(), before);
    assert_eq!(disk.read("activity-log.json").unwrap(), bytes);
    issue(
        &mut store,
        tracking::Command::Pause {
            reason: "manual".into(),
            cutoff: None,
        },
        2000,
        "unused",
    )
    .unwrap();
    assert_eq!(store.snapshot().activities.len(), 1);
}
#[test]
fn uncertain_write_blocks_further_commands_until_reopen() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk.clone(), 0, "open-1").unwrap();
    start(&mut store);
    disk.fail("activity-log.json", true, false);
    assert!(issue(
        &mut store,
        tracking::Command::Start {
            name: "B".into(),
            todo_id: None
        },
        1000,
        "b"
    )
    .is_err());
    assert!(store.is_blocked());
    assert_eq!(store.snapshot().activities.len(), 1);
    assert!(store.checkpoint(2000).is_err());
    let restored = TrackingStore::open(disk, 3000, "open-2").unwrap();
    assert_eq!(restored.snapshot().activities.len(), 2);
    assert!(restored.snapshot().current().is_none());
}
#[test]
fn sleep_with_full_disk_stops_memory_clock_and_recovery_caps_at_checkpoint() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk.clone(), 0, "open-1").unwrap();
    start(&mut store);
    store.checkpoint(30_000).unwrap();
    disk.fail("activity-log.json", false, false);
    assert!(store.interrupt(60_000, "sleep").is_err());
    assert!(store.is_blocked());
    assert!(store.snapshot().current().is_none());
    assert_eq!(
        store.snapshot().activities[0]
            .duration(1_000_000, None)
            .unwrap(),
        60_000
    );
    assert!(issue(
        &mut store,
        tracking::Command::Resume {
            id: Some("a".into())
        },
        1_000_000,
        "unused"
    )
    .is_err());
    let restored = TrackingStore::open(disk, 1_000_000, "open-2").unwrap();
    assert_eq!(
        restored.snapshot().activities[0]
            .duration(1_000_000, None)
            .unwrap(),
        30_000
    );
}
#[test]
fn damaged_original_is_retained_and_future_schema_never_uses_old_backup() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk.clone(), 0, "open-1").unwrap();
    start(&mut store);
    store.checkpoint(30_000).unwrap();
    disk.put("activity-log.json", b"{broken");
    let restored = TrackingStore::open(disk.clone(), 60_000, "open-2").unwrap();
    assert!(restored.notice.is_some());
    assert_eq!(
        disk.read("activity-log.json.damaged-open-2")
            .unwrap()
            .unwrap(),
        b"{broken"
    );
    disk.put("activity-log.json", br#"{"version":99,"activities":[]}"#);
    let before = disk.files.lock().unwrap().clone();
    assert!(TrackingStore::open(disk.clone(), 70_000, "open-3").is_err());
    assert_eq!(*disk.files.lock().unwrap(), before);
}
#[test]
fn external_edit_is_never_silently_overwritten() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk.clone(), 0, "open-1").unwrap();
    start(&mut store);
    disk.put("activity-log.json", b"external");
    assert!(store.checkpoint(30_000).is_err());
    assert!(store.is_blocked());
    assert_eq!(
        disk.read("activity-log.json").unwrap().unwrap(),
        b"external"
    );
}
#[test]
fn clock_gap_stops_at_previous_checkpoint_instead_of_counting_sleep() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk, 0, "open-1").unwrap();
    start(&mut store);
    store.checkpoint(30_000).unwrap();
    store.checkpoint(1_000_000).unwrap();
    assert!(store.snapshot().current().is_none());
    assert_eq!(
        store.snapshot().activities[0]
            .duration(1_000_000, None)
            .unwrap(),
        30_000
    );
}
fn todo_issue(
    store: &mut TodosStore,
    command: todos::Command,
    id: &str,
) -> Result<todos::Document, String> {
    store.command(
        &command,
        store.snapshot().revision,
        todos::Context {
            now: 0,
            local_day: "2026-11-14",
            new_id: id,
        },
    )
}
fn completed_todo(store: &mut TodosStore) {
    todo_issue(
        store,
        todos::Command::Create(todos::Fields {
            text: "阅读".into(),
            notes: "原备注".into(),
            urgent: false,
            deadline: None,
        }),
        "a",
    )
    .unwrap();
    todo_issue(
        store,
        todos::Command::Complete {
            id: "a".into(),
            done: true,
        },
        "unused",
    )
    .unwrap();
}
#[test]
fn archive_recovers_after_process_termination_at_every_commit_boundary() {
    for stage in [
        "catch-archive-pending.json",
        "archive.json",
        "todos.json",
        "cleanup",
    ] {
        let disk = Arc::new(Memory::default());
        let mut store = TodosStore::open(disk.clone()).unwrap();
        completed_todo(&mut store);
        disk.fail(
            if stage == "cleanup" {
                "catch-archive-pending.json"
            } else {
                stage
            },
            true,
            true,
        );
        if stage == "cleanup" {
            // Trigger only on remove, after the journal was already written.
            let before = store.snapshot().clone();
            let journal = serde_json::json!({"version":1,"before":before.todos,"after":[],"archiveBefore":[],"archiveAfter":before.todos});
            disk.put(
                "catch-archive-pending.json",
                &serde_json::to_vec(&journal).unwrap(),
            );
            assert!(catch_unwind(AssertUnwindSafe(|| TodosStore::open(disk.clone()))).is_err());
        } else {
            assert!(catch_unwind(AssertUnwindSafe(|| todo_issue(
                &mut store,
                todos::Command::Archive,
                "unused"
            )))
            .is_err());
        }
        let restored = TodosStore::open(disk.clone()).unwrap();
        assert!(restored.snapshot().todos.is_empty(), "{stage}");
        assert_eq!(restored.snapshot().archive.len(), 1, "{stage}");
        assert_eq!(restored.snapshot().archive[0].notes, "原备注");
        assert!(disk.read("catch-archive-pending.json").unwrap().is_none());
        assert_eq!(
            TodosStore::open(disk).unwrap().snapshot(),
            restored.snapshot()
        );
    }
}
#[test]
fn archive_recovery_refuses_external_mutation() {
    let disk = Arc::new(Memory::default());
    let mut store = TodosStore::open(disk.clone()).unwrap();
    completed_todo(&mut store);
    disk.fail("catch-archive-pending.json", true, true);
    assert!(catch_unwind(AssertUnwindSafe(|| todo_issue(
        &mut store,
        todos::Command::Archive,
        "unused"
    )))
    .is_err());
    disk.put(
        "archive.json",
        br#"[{"id":"external","text":"external","done":true}]"#,
    );
    let before = disk.files.lock().unwrap().clone();
    assert!(TodosStore::open(disk.clone()).is_err());
    assert_eq!(*disk.files.lock().unwrap(), before);
}
#[test]
fn filesystem_replaces_atomically_and_profile_lock_lasts_until_drop() {
    let dir = tempfile::tempdir().unwrap();
    let lock = ProfileLock::acquire(dir.path()).unwrap();
    assert!(ProfileLock::acquire(dir.path()).is_err());
    let disk = FileDisk::new(dir.path()).unwrap();
    disk.write_atomic("test.json", b"old").unwrap();
    disk.write_atomic("test.json", b"new").unwrap();
    assert_eq!(disk.read("test.json").unwrap().unwrap(), b"new");
    assert!(disk.write_atomic("../outside.json", b"bad").is_err());
    drop(lock);
    assert!(ProfileLock::acquire(dir.path()).is_ok());
}

#[test]
fn future_trash_schema_is_protected_even_with_a_readable_backup() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk.clone(), 0, "open-1").unwrap();
    start(&mut store);
    store.checkpoint(30_000).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&disk.read("activity-log.json").unwrap().unwrap()).unwrap();
    value["trash"] = serde_json::json!([{"kind":"future-kind"}]);
    disk.put("activity-log.json", &serde_json::to_vec(&value).unwrap());
    let before = disk.files.lock().unwrap().clone();
    assert!(TrackingStore::open(disk.clone(), 60_000, "open-2").is_err());
    assert_eq!(*disk.files.lock().unwrap(), before);
}

#[test]
fn migration_backup_is_byte_exact_and_a_failed_backup_never_changes_main() {
    let disk = Arc::new(Memory::default());
    let mut store = TrackingStore::open(disk.clone(), 0, "open-1").unwrap();
    start(&mut store);
    store.interrupt(30_000, "manual").unwrap();
    let mut legacy = serde_json::to_value(store.snapshot()).unwrap();
    legacy["version"] = serde_json::json!(1);
    legacy.as_object_mut().unwrap().remove("sessionVersion");
    legacy["activities"][0]
        .as_object_mut()
        .unwrap()
        .remove("sessions");
    legacy["activities"][0]
        .as_object_mut()
        .unwrap()
        .remove("nextSessionIndex");
    let original = serde_json::to_vec_pretty(&legacy).unwrap();
    disk.put("activity-log.json", &original);
    disk.fail("activity-log.json.v1-backup-failed.json", false, false);
    assert!(TrackingStore::open(disk.clone(), 60_000, "failed").is_err());
    assert_eq!(disk.read("activity-log.json").unwrap().unwrap(), original);
    let migrated = TrackingStore::open(disk.clone(), 60_000, "succeeded").unwrap();
    assert_eq!(
        disk.read("activity-log.json.v1-backup-succeeded.json")
            .unwrap()
            .unwrap(),
        original
    );
    assert_eq!(
        migrated.snapshot().activities[0].sessions.as_ref().unwrap()[0].id,
        "a:0"
    );
    let before = disk.files.lock().unwrap().clone();
    TrackingStore::open(disk.clone(), 120_000, "again").unwrap();
    assert_eq!(*disk.files.lock().unwrap(), before);
}

#[test]
fn missing_main_files_with_backups_are_not_treated_as_empty_profiles() {
    let disk = Arc::new(Memory::default());
    disk.put("todos.json.bak", b"[]");
    assert!(TodosStore::open(disk.clone()).is_err());
    disk.put("activity-log.json.bak", b"{}");
    assert!(TrackingStore::open(disk, 0, "open-1").is_err());
}
