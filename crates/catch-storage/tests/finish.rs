use catch_core::{recurrence, todos, tracking};
use catch_storage::{
    finish::{self, Candidate, Receipt},
    Disk, Result, TodosStore, TrackingStore,
};
use std::{
    collections::BTreeMap,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
};
#[derive(Clone, Copy)]
struct Fault {
    index: usize,
    after: bool,
    crash: bool,
}
#[derive(Default)]
struct Memory {
    files: Mutex<BTreeMap<String, Vec<u8>>>,
    events: Mutex<Vec<String>>,
    fault: Mutex<Option<Fault>>,
}
impl Memory {
    fn event(&self, label: String) -> (usize, Option<Fault>) {
        let mut events = self.events.lock().unwrap();
        let index = events.len();
        events.push(label);
        let fault = self.fault.lock().unwrap().filter(|f| f.index == index);
        (index, fault)
    }
    fn trip(fault: Option<Fault>, after: bool) -> Result<()> {
        if let Some(fault) = fault.filter(|fault| fault.after == after) {
            if fault.crash {
                panic!("simulated process termination");
            }
            return Err("injected I/O failure".into());
        }
        Ok(())
    }
    fn arm(&self, fault: Option<Fault>) {
        self.events.lock().unwrap().clear();
        *self.fault.lock().unwrap() = fault;
    }
    fn raw(&self, name: &str) -> Option<Vec<u8>> {
        self.files.lock().unwrap().get(name).cloned()
    }
}
impl Disk for Memory {
    fn read(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let (_, fault) = self.event(format!("read:{name}"));
        Self::trip(fault, false)?;
        let bytes = self.raw(name);
        Self::trip(fault, true)?;
        Ok(bytes)
    }
    fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let (_, fault) = self.event(format!("write:{name}"));
        Self::trip(fault, false)?;
        self.files.lock().unwrap().insert(name.into(), bytes.into());
        Self::trip(fault, true)
    }
    fn remove(&self, name: &str) -> Result<()> {
        let (_, fault) = self.event(format!("remove:{name}"));
        Self::trip(fault, false)?;
        self.files.lock().unwrap().remove(name);
        Self::trip(fault, true)
    }
}
fn fixture() -> (Arc<Memory>, TrackingStore, TodosStore, Candidate) {
    let disk = Arc::new(Memory::default());
    let mut todos = TodosStore::open(disk.clone()).unwrap();
    todos
        .recurrence_command(
            &recurrence::Command::Create {
                fields: todos::Fields {
                    text: "work".into(),
                    notes: "preserve notes".into(),
                    urgent: false,
                    deadline: None,
                },
                resources: vec![],
                schedule: recurrence::Schedule::Daily,
                starts_on: "2026-11-14".into(),
            },
            0,
            "2026-11-14",
            "rule",
        )
        .unwrap();
    let id = todos.snapshot().todos[0].id.clone();
    let mut tracking = TrackingStore::open(disk.clone(), 0, "startup").unwrap();
    tracking
        .command(
            &tracking::Command::Start {
                name: "work".into(),
                todo_id: Some(id.clone()),
                resources: vec![],
            },
            0,
            tracking::Context {
                now: 0,
                new_id: "matter",
            },
        )
        .unwrap();
    let next_tracking = tracking
        .snapshot()
        .finish(
            "matter",
            Some("matter:0"),
            Some("final draft"),
            tracking.snapshot().revision,
            tracking::Context {
                now: 1000,
                new_id: "unused",
            },
        )
        .unwrap();
    let next_todos = todos
        .snapshot()
        .apply(
            &todos::Command::Complete { id, done: true },
            todos.snapshot().revision,
            todos::Context {
                now: 1000,
                local_day: "2026-11-14",
                new_id: "unused",
            },
        )
        .unwrap();
    let rules = todos
        .recurrences()
        .after_todo_change(todos.snapshot(), &next_todos, "2026-11-14")
        .unwrap();
    // Exercise replacement/backup of a receipt file without losing prior IDs.
    disk.write_atomic(
        "catch-finish-receipts.json",
        &serde_json::to_vec(&vec![Receipt {
            request_id: "older".into(),
            fingerprint: vec![1],
            response: vec![2],
        }])
        .unwrap(),
    )
    .unwrap();
    disk.arm(None);
    (
        disk,
        tracking,
        todos,
        Candidate {
            tracking: next_tracking,
            todos: next_todos,
            rules,
        },
    )
}
fn receipt() -> Receipt {
    Receipt {
        request_id: "finish".into(),
        fingerprint: b"same request".to_vec(),
        response: b"original response".to_vec(),
    }
}
fn clone_candidate(candidate: &Candidate) -> Candidate {
    Candidate {
        tracking: candidate.tracking.clone(),
        todos: candidate.todos.clone(),
        rules: candidate.rules.clone(),
    }
}
fn state(disk: &Memory) -> (tracking::Document, Vec<todos::Todo>, recurrence::Document) {
    (
        serde_json::from_slice(&disk.raw("activity-log.json").unwrap()).unwrap(),
        serde_json::from_slice(&disk.raw("todos.json").unwrap()).unwrap(),
        serde_json::from_slice(&disk.raw("catch-recurrences.json").unwrap()).unwrap(),
    )
}
#[test]
fn errors_before_and_after_every_io_boundary_never_report_partial_success() {
    let (disk, mut tracking, mut todos, candidate) = fixture();
    finish::commit(&mut tracking, &mut todos, candidate, receipt()).unwrap();
    let events = disk.events.lock().unwrap().clone();
    assert!(events.iter().any(|e| e == "write:activity-log.json.bak"));
    assert!(events.iter().any(|e| e == "write:todos.json.bak"));
    assert!(events
        .iter()
        .any(|e| e == "write:catch-recurrences.json.bak"));
    assert!(events
        .iter()
        .any(|e| e == "write:catch-finish-receipts.json.bak"));
    for (index, event) in events.iter().enumerate() {
        for after in [false, true] {
            let (disk, mut tracking, mut todos, candidate) = fixture();
            let before = state(&disk);
            let expected = (
                candidate.tracking.clone(),
                candidate.todos.todos.clone(),
                candidate.rules.clone(),
            );
            disk.arm(Some(Fault {
                index,
                after,
                crash: false,
            }));
            let result = finish::commit(&mut tracking, &mut todos, candidate, receipt());
            disk.arm(None);
            finish::recover(disk.as_ref()).unwrap();
            let actual = state(&disk);
            let committed = finish::lookup(disk.as_ref(), "finish").unwrap().is_some();
            assert_eq!(
                actual,
                if committed { expected } else { before },
                "{event}, after={after}"
            );
            if result.is_ok() {
                assert!(committed, "{event}, after={after}");
            }
            assert!(finish::lookup(disk.as_ref(), "older").unwrap().is_some());
            assert!(disk.raw("catch-finish-pending.json").is_none());
        }
    }
}
#[test]
fn process_termination_at_every_io_boundary_recovers_whole_documents_and_receipt() {
    let (disk, mut tracking, mut todos, candidate) = fixture();
    finish::commit(&mut tracking, &mut todos, candidate, receipt()).unwrap();
    let events = disk.events.lock().unwrap().clone();
    for (index, event) in events.iter().enumerate() {
        for after in [false, true] {
            let (disk, mut tracking, mut todos, candidate) = fixture();
            let before = state(&disk);
            let expected = (
                candidate.tracking.clone(),
                candidate.todos.todos.clone(),
                candidate.rules.clone(),
            );
            disk.arm(Some(Fault {
                index,
                after,
                crash: true,
            }));
            assert!(catch_unwind(AssertUnwindSafe(|| finish::commit(
                &mut tracking,
                &mut todos,
                candidate,
                receipt()
            )))
            .is_err());
            disk.arm(None);
            let committed = disk.raw("catch-finish-pending.json").is_some()
                || finish::lookup(disk.as_ref(), "finish").unwrap().is_some();
            // Either repository can be the first reader after a crash. Its open
            // must finish the cross-store journal before loading its own state.
            if committed {
                if index % 2 == 0 {
                    TodosStore::open(disk.clone()).unwrap();
                } else {
                    TrackingStore::open(disk.clone(), 2000, "restart").unwrap();
                }
            } else {
                finish::recover(disk.as_ref()).unwrap();
            }
            finish::recover(disk.as_ref()).unwrap();
            assert_eq!(
                state(&disk),
                if committed { expected } else { before },
                "{event}, after={after}"
            );
            assert_eq!(
                finish::lookup(disk.as_ref(), "finish").unwrap().is_some(),
                committed
            );
            assert!(finish::lookup(disk.as_ref(), "older").unwrap().is_some());
            assert!(disk.raw("catch-finish-pending.json").is_none());
        }
    }
}
#[test]
fn pending_recovery_refuses_external_changes_or_invalid_intent_without_overwriting() {
    let (disk, mut tracking, mut todos, candidate) = fixture();
    finish::commit(
        &mut tracking,
        &mut todos,
        clone_candidate(&candidate),
        receipt(),
    )
    .unwrap();
    let index = disk
        .events
        .lock()
        .unwrap()
        .iter()
        .position(|e| e == "write:todos.json")
        .unwrap();
    let (disk, mut tracking, mut todos, candidate) = fixture();
    disk.arm(Some(Fault {
        index,
        after: false,
        crash: true,
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| finish::commit(
        &mut tracking,
        &mut todos,
        candidate,
        receipt()
    )))
    .is_err());
    disk.arm(None);
    let pending = disk.raw("catch-finish-pending.json").unwrap();
    disk.files
        .lock()
        .unwrap()
        .insert("todos.json".into(), b"[] external".to_vec());
    let original = disk.files.lock().unwrap().clone();
    assert!(finish::recover(disk.as_ref()).is_err());
    assert!(TodosStore::open(disk.clone()).is_err());
    assert!(TrackingStore::open(disk.clone(), 2000, "restart").is_err());
    assert_eq!(*disk.files.lock().unwrap(), original);
    let mut journal: serde_json::Value = serde_json::from_slice(&pending).unwrap();
    journal["changes"][0]["file"] = "archive.json".into();
    disk.files.lock().unwrap().insert(
        "catch-finish-pending.json".into(),
        serde_json::to_vec(&journal).unwrap(),
    );
    let original = disk.files.lock().unwrap().clone();
    assert!(finish::recover(disk.as_ref()).is_err());
    assert_eq!(*disk.files.lock().unwrap(), original);
}
#[test]
fn external_edits_before_commit_leave_both_documents_and_receipts_untouched() {
    for file in [
        "activity-log.json",
        "todos.json",
        "catch-recurrences.json",
        "archive.json",
    ] {
        let (disk, mut tracking, mut todos, candidate) = fixture();
        disk.files
            .lock()
            .unwrap()
            .entry(file.into())
            .or_default()
            .push(b' ');
        let before = disk.files.lock().unwrap().clone();
        assert!(
            finish::commit(&mut tracking, &mut todos, candidate, receipt()).is_err(),
            "{file}"
        );
        assert_eq!(*disk.files.lock().unwrap(), before);
        assert!(finish::lookup(disk.as_ref(), "finish").unwrap().is_none());
    }
}
