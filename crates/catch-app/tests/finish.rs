use catch_app::{Context, ErrorCode, FinishRequest, Request, Service, UserCommand};
use catch_core::{recurrence, todos, tracking};
use catch_storage::{Disk, FileDisk};
use std::sync::{Arc, Mutex};
fn context(id: &str, now: i64) -> Context<'_> {
    Context {
        now,
        local_day: "2026-11-14",
        new_id: id,
    }
}
fn request(service: &Service, id: &str, complete_todo: bool) -> Request {
    let snapshot = service.snapshot();
    Request {
        request_id: id.into(),
        expected_revision: snapshot.tracking.revision,
        operation: UserCommand::Finish(FinishRequest {
            activity_id: "matter".into(),
            session_id: Some("matter:0".into()),
            progress: Some(" final draft ".into()),
            complete_todo,
            expected_todos_revision: snapshot.todos.revision,
        }),
    }
}
fn start(service: &mut Service, associated: bool) {
    service
        .execute(
            Request {
                request_id: "start".into(),
                expected_revision: service.snapshot().tracking.revision,
                operation: UserCommand::Tracking(tracking::Command::Start {
                    name: "work".into(),
                    todo_id: associated.then(|| "todo".into()),
                    resources: vec![],
                }),
            },
            context("matter", 0),
        )
        .unwrap();
}
fn fixture(associated: bool) -> (tempfile::TempDir, Arc<FileDisk>, Service) {
    let dir = tempfile::tempdir().unwrap();
    let disk = Arc::new(FileDisk::new(dir.path()).unwrap());
    disk.write_atomic(
        "todos.json",
        br#"[{"id":"todo","text":"work","done":false,"custom":"keep"}]"#,
    )
    .unwrap();
    let mut service = Service::open(disk.clone(), context("startup", 0)).unwrap();
    start(&mut service, associated);
    (dir, disk, service)
}
#[test]
fn plain_finish_payload_defaults_optional_fields_and_complete_flag() {
    let command: UserCommand = serde_json::from_value(serde_json::json!({
        "group": "finish", "command": {"activityId": "matter", "expectedTodosRevision": 4}
    }))
    .unwrap();
    let UserCommand::Finish(payload) = command else {
        panic!("wrong command");
    };
    assert!(!payload.complete_todo);
    assert!(payload.session_id.is_none());
    assert!(payload.progress.is_none());
    assert_eq!(payload.expected_todos_revision, 4);
}

#[test]
fn a_real_legacy_finish_receipt_migrates_and_replays_its_original_snapshot() {
    let (_dir, disk, mut service) = fixture(true);
    let finish = request(&service, "legacy-finish", true);
    let result = service
        .execute(finish.clone(), context("unused", 1000))
        .unwrap();
    let original = serde_json::to_vec(&serde_json::json!([{
        "request_id": finish.request_id,
        "fingerprint": serde_json::to_vec(&finish).unwrap(),
        "response": serde_json::to_vec(&result).unwrap()
    }]))
    .unwrap();
    drop(service);
    disk.write_atomic("catch-finish-receipts.json", &original)
        .unwrap();
    let mut restarted = Service::open(disk.clone(), context("restart", 2000)).unwrap();
    let replay = restarted.execute(finish, context("unused", 3000)).unwrap();
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    assert_eq!(
        disk.read("catch-finish-receipts.json.bak")
            .unwrap()
            .unwrap(),
        original
    );
}
#[test]
fn finish_commits_draft_end_and_todo_and_replays_exactly_after_restart() {
    let (_dir, disk, mut service) = fixture(true);
    let finish = request(&service, "finish", true);
    let result = service
        .execute(finish.clone(), context("unused", 1000))
        .unwrap();
    assert_eq!(result.tracking.revision, 2);
    assert_eq!(
        result.tracking.activities[0].status,
        tracking::Status::Completed
    );
    assert_eq!(
        result.tracking.activities[0].sessions.as_ref().unwrap()[0].progress,
        "final draft"
    );
    assert!(result.todos.todos[0].done);
    assert_eq!(result.todos.todos[0].extra["custom"], "keep");
    let expected = serde_json::to_value(&result).unwrap();
    assert_eq!(
        serde_json::to_value(
            service
                .execute(finish.clone(), context("unused", 2000))
                .unwrap()
        )
        .unwrap(),
        expected
    );
    let mut restarted = Service::open(disk, context("restart", 3000)).unwrap();
    assert_eq!(
        serde_json::to_value(
            restarted
                .execute(finish.clone(), context("unused", 4000))
                .unwrap()
        )
        .unwrap(),
        expected
    );
    let mut collision = finish;
    let current = restarted.snapshot();
    let replay = restarted
        .execute(collision.clone(), context("unused", 4000))
        .unwrap();
    let reply = catch_app::Reply::from_result(Ok(replay), current.clone());
    assert_eq!(reply.state.todos.revision, current.todos.revision);
    assert_eq!(
        serde_json::to_value(reply.state).unwrap(),
        serde_json::to_value(current).unwrap()
    );
    if let UserCommand::Finish(ref mut operation) = collision.operation {
        operation.complete_todo = false;
    }
    assert_eq!(
        restarted
            .execute(collision, context("unused", 4000))
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert!(restarted.snapshot().todos.todos[0].done);
}
#[test]
fn both_revisions_and_session_identity_are_checked_before_any_write() {
    let (_dir, _disk, mut service) = fixture(true);
    let original = serde_json::to_value(service.snapshot()).unwrap();
    for tracking_stale in [false, true] {
        let mut finish = request(&service, "stale", true);
        if tracking_stale {
            finish.expected_revision += 1;
        } else if let UserCommand::Finish(ref mut payload) = finish.operation {
            payload.expected_todos_revision += 1;
        }
        assert_eq!(
            service
                .execute(finish, context("unused", 1000))
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
    }
    let mut finish = request(&service, "wrong-session", true);
    if let UserCommand::Finish(ref mut payload) = finish.operation {
        payload.session_id = Some("foreign:0".into());
    }
    assert_eq!(
        service
            .execute(finish, context("unused", 1000))
            .unwrap_err()
            .code,
        ErrorCode::Validation
    );
    assert_eq!(serde_json::to_value(service.snapshot()).unwrap(), original);
}
#[test]
fn absent_completed_deleted_and_missing_links_require_finish_without_completion() {
    for state in ["absent", "done", "deleted", "missing"] {
        let (_dir, _disk, mut service) = fixture(state != "absent");
        let operations = match state {
            "done" => vec![todos::Command::Complete {
                id: "todo".into(),
                done: true,
            }],
            "deleted" => vec![todos::Command::Trash { id: "todo".into() }],
            "missing" => vec![
                todos::Command::Trash { id: "todo".into() },
                todos::Command::Delete { id: "todo".into() },
            ],
            _ => vec![],
        };
        for (index, command) in operations.into_iter().enumerate() {
            service
                .execute(
                    Request {
                        request_id: format!("todo-{index}"),
                        expected_revision: service.snapshot().todos.revision,
                        operation: UserCommand::Todos(command),
                    },
                    context("unused", 500),
                )
                .unwrap();
        }
        let finish = request(&service, "finish", true);
        assert_eq!(
            service
                .execute(finish, context("unused", 1000))
                .unwrap_err()
                .code,
            ErrorCode::Validation,
            "{state}"
        );
        assert_ne!(
            service.snapshot().tracking.activities[0].status,
            tracking::Status::Completed
        );
        let mut finish = request(&service, "finish-only", false);
        if let UserCommand::Finish(ref mut payload) = finish.operation {
            payload.progress = None;
            payload.session_id = None;
        }
        assert!(
            service.execute(finish, context("unused", 1000)).is_ok(),
            "{state}"
        );
    }
}
#[test]
fn next_session_empty_progress_is_replayable_but_old_session_is_not_reusable() {
    let (_dir, _disk, mut service) = fixture(false);
    let next = Request {
        request_id: "next".into(),
        expected_revision: 1,
        operation: UserCommand::Tracking(tracking::Command::NextSession {
            activity_id: "matter".into(),
            session_id: "matter:0".into(),
            progress: "".into(),
        }),
    };
    service
        .execute(next.clone(), context("unused", 1000))
        .unwrap();
    service
        .execute(next.clone(), context("unused", 2000))
        .unwrap();
    let mut duplicate = next;
    duplicate.request_id = "duplicate".into();
    assert_eq!(
        service
            .execute(duplicate.clone(), context("unused", 2000))
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    duplicate.expected_revision = service.snapshot().tracking.revision;
    assert_eq!(
        service
            .execute(duplicate, context("unused", 2000))
            .unwrap_err()
            .code,
        ErrorCode::Validation
    );
    assert_eq!(
        service.snapshot().tracking.activities[0]
            .sessions
            .as_ref()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn recurring_completion_updates_cursor_in_the_same_finish() {
    let dir = tempfile::tempdir().unwrap();
    let disk = Arc::new(FileDisk::new(dir.path()).unwrap());
    let mut service = Service::open(disk.clone(), context("startup", 0)).unwrap();
    service
        .execute(
            Request {
                request_id: "rule".into(),
                expected_revision: 0,
                operation: UserCommand::Recurrence(catch_app::RecurrenceRequest {
                    expected_todos_revision: 0,
                    command: recurrence::Command::Create {
                        fields: todos::Fields {
                            text: "work".into(),
                            notes: "".into(),
                            urgent: false,
                            deadline: None,
                        },
                        resources: vec![],
                        schedule: recurrence::Schedule::Daily,
                        starts_on: "2026-11-14".into(),
                    },
                }),
            },
            context("rule", 0),
        )
        .unwrap();
    let todo_id = service.snapshot().todos.todos[0].id.clone();
    service
        .execute(
            Request {
                request_id: "start".into(),
                expected_revision: 0,
                operation: UserCommand::Tracking(tracking::Command::Start {
                    name: "work".into(),
                    todo_id: Some(todo_id),
                    resources: vec![],
                }),
            },
            context("matter", 0),
        )
        .unwrap();
    let finish = request(&service, "finish", true);
    service.execute(finish, context("unused", 1000)).unwrap();
    let expected = service.snapshot().recurrences;
    let reopened = Service::open(disk, context("restart", 2000)).unwrap();
    assert_eq!(reopened.snapshot().recurrences, expected);
    assert_eq!(reopened.snapshot().todos.todos.len(), 1);
    assert!(reopened.snapshot().todos.todos[0].done);
}
struct FailingDisk {
    inner: FileDisk,
    fail: Mutex<bool>,
}
impl Disk for FailingDisk {
    fn read(&self, name: &str) -> catch_storage::Result<Option<Vec<u8>>> {
        self.inner.read(name)
    }
    fn write_atomic(&self, name: &str, bytes: &[u8]) -> catch_storage::Result<()> {
        if name == "todos.json" && *self.fail.lock().unwrap() {
            return Err("full disk".into());
        }
        self.inner.write_atomic(name, bytes)
    }
    fn remove(&self, name: &str) -> catch_storage::Result<()> {
        self.inner.remove(name)
    }
}
#[test]
fn unresolved_partial_finish_blocks_all_writes_and_restart_recovers_and_replays() {
    let dir = tempfile::tempdir().unwrap();
    let disk = Arc::new(FailingDisk {
        inner: FileDisk::new(dir.path()).unwrap(),
        fail: Mutex::new(false),
    });
    disk.write_atomic(
        "todos.json",
        br#"[{"id":"todo","text":"work","done":false}]"#,
    )
    .unwrap();
    let mut service = Service::open(disk.clone(), context("startup", 0)).unwrap();
    start(&mut service, true);
    let finish = request(&service, "finish", true);
    *disk.fail.lock().unwrap() = true;
    assert_eq!(
        service
            .execute(finish.clone(), context("unused", 1000))
            .unwrap_err()
            .code,
        ErrorCode::StorageFailure
    );
    assert!(service.snapshot().storage_blocked);
    assert!(service.interrupt(2000, "sleep").is_err());
    assert!(service.checkpoint(2000).is_err());
    assert!(service
        .execute(request(&service, "other", false), context("unused", 2000))
        .is_err());
    assert!(disk.read("catch-finish-pending.json").unwrap().is_some());
    *disk.fail.lock().unwrap() = false;
    let mut recovered = Service::open(disk.clone(), context("restart", 3000)).unwrap();
    assert_eq!(
        recovered.snapshot().tracking.activities[0].status,
        tracking::Status::Completed
    );
    assert!(recovered.snapshot().todos.todos[0].done);
    let replay = recovered.execute(finish, context("unused", 4000)).unwrap();
    assert_eq!(
        replay.tracking.activities[0].sessions.as_ref().unwrap()[0].progress,
        "final draft"
    );
    assert!(!recovered.snapshot().storage_blocked);
    assert!(disk.read("catch-finish-pending.json").unwrap().is_none());
}

#[test]
fn many_finishes_keep_recent_exact_replays_and_reject_compacted_requests_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let disk = Arc::new(FileDisk::new(dir.path()).unwrap());
    let mut service = Service::open(disk.clone(), context("startup", 0)).unwrap();
    let mut first = None;
    let mut last = None;
    for index in 0..40 {
        let id = format!("matter-{index}");
        service
            .execute(
                Request {
                    request_id: format!("start-{index}"),
                    expected_revision: service.snapshot().tracking.revision,
                    operation: UserCommand::Tracking(tracking::Command::Start {
                        name: id.clone(),
                        todo_id: None,
                        resources: vec![],
                    }),
                },
                context(&id, index * 2000),
            )
            .unwrap();
        let finish = Request {
            request_id: format!("finish-{index}"),
            expected_revision: service.snapshot().tracking.revision,
            operation: UserCommand::Finish(FinishRequest {
                activity_id: id,
                session_id: None,
                progress: None,
                complete_todo: false,
                expected_todos_revision: 0,
            }),
        };
        let result = service
            .execute(finish.clone(), context("unused", index * 2000 + 1000))
            .unwrap();
        if index == 0 {
            first = Some(finish.clone());
        }
        last = Some((finish, serde_json::to_value(result).unwrap()));
    }
    let first = first.unwrap();
    let (last, expected) = last.unwrap();
    let before = serde_json::to_value(service.snapshot()).unwrap();
    let error = service
        .execute(first.clone(), context("unused", 90000))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert!(error.message.contains("历史响应已压缩"));
    assert_eq!(serde_json::to_value(service.snapshot()).unwrap(), before);
    let mut restarted = Service::open(disk, context("restart", 90000)).unwrap();
    assert_eq!(
        restarted
            .execute(first.clone(), context("unused", 90000))
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let mut collision = first;
    if let UserCommand::Finish(ref mut operation) = collision.operation {
        operation.progress = Some("不同参数".into());
    }
    let error = restarted
        .execute(collision, context("unused", 90000))
        .unwrap_err();
    assert!(error.message.contains("其他操作"));
    assert_eq!(
        serde_json::to_value(restarted.execute(last, context("unused", 90000)).unwrap()).unwrap(),
        expected
    );
    assert_eq!(restarted.snapshot().tracking.activities.len(), 40);
}

struct CountReceiptReads {
    inner: FileDisk,
    reads: std::sync::atomic::AtomicUsize,
}
impl Disk for CountReceiptReads {
    fn read(&self, name: &str) -> catch_storage::Result<Option<Vec<u8>>> {
        if name == "catch-finish-receipts.json" {
            self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        self.inner.read(name)
    }
    fn write_atomic(&self, name: &str, bytes: &[u8]) -> catch_storage::Result<()> {
        self.inner.write_atomic(name, bytes)
    }
    fn remove(&self, name: &str) -> catch_storage::Result<()> {
        self.inner.remove(name)
    }
}
#[test]
fn ordinary_commands_and_exact_replays_do_not_read_the_receipt_file() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let disk = Arc::new(CountReceiptReads {
        inner: FileDisk::new(dir.path()).unwrap(),
        reads: AtomicUsize::new(0),
    });
    let mut service = Service::open(disk.clone(), context("startup", 0)).unwrap();
    start(&mut service, false);
    let finish = request(&service, "finish", false);
    let result = service
        .execute(finish.clone(), context("unused", 1000))
        .unwrap();
    disk.reads.store(0, Ordering::SeqCst);
    for index in 0..50 {
        service
            .execute(
                Request {
                    request_id: format!("goal-{index}"),
                    expected_revision: service.snapshot().growth.revision,
                    operation: UserCommand::Growth(catch_core::growth::Command::SetGoal {
                        minutes: Some(index + 1),
                    }),
                },
                context("unused", 2000),
            )
            .unwrap();
    }
    let replay = service.execute(finish, context("unused", 3000)).unwrap();
    assert_eq!(
        serde_json::to_value(&replay).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    let reply = catch_app::Reply::from_result(Ok(replay), service.snapshot());
    assert!(reply.ok);
    assert_eq!(reply.state.growth.goal_minutes, Some(50));
    assert_eq!(disk.reads.load(Ordering::SeqCst), 0);
}
