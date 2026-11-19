use catch_app::{Context, ErrorCode, Request, Service, Snapshot, UserCommand};
use catch_core::{
    growth,
    tracking::{self, Command, Status},
};
use catch_storage::Disk;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct MemoryDisk {
    inner: Mutex<Memory>,
}
#[derive(Default)]
struct Memory {
    files: BTreeMap<String, Vec<u8>>,
    failure: Option<(String, bool)>,
    writes: Vec<String>,
}
impl Disk for MemoryDisk {
    fn read(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(self.inner.lock().unwrap().files.get(key).cloned())
    }
    fn remove(&self, key: &str) -> Result<(), String> {
        self.inner.lock().unwrap().files.remove(key);
        Ok(())
    }
    fn write_atomic(&self, key: &str, bytes: &[u8]) -> Result<(), String> {
        let mut m = self.inner.lock().unwrap();
        let fault = m
            .failure
            .as_ref()
            .filter(|(name, _)| name == key)
            .map(|(_, after)| *after);
        if fault == Some(false) {
            m.failure = None;
            return Err("injected before write".into());
        }
        m.files.insert(key.into(), bytes.to_vec());
        m.writes.push(key.into());
        if fault == Some(true) {
            m.failure = None;
            return Err("injected after write".into());
        }
        Ok(())
    }
}
fn clock(now: i64, id: &str) -> Context<'_> {
    Context {
        now: now * 60_000,
        local_day: "2026-11-14",
        new_id: id,
    }
}
fn t(n: i64) -> String {
    tracking::iso(n * 60_000).unwrap()
}
fn request(id: &str, revision: u64, kind: &str, payload: Value) -> Request {
    Request {
        request_id: id.into(),
        expected_revision: revision,
        operation: UserCommand::Tracking(
            serde_json::from_value(json!({"type":kind,"payload":payload})).unwrap(),
        ),
    }
}
fn exec(service: &mut Service, id: &str, kind: &str, payload: Value, now: i64) -> Snapshot {
    let request = request(id, service.snapshot().tracking.revision, kind, payload);
    service.execute(request, clock(now, id)).unwrap()
}
fn setup(kind: &str) -> (Arc<MemoryDisk>, Service, Request, i64) {
    let disk = Arc::new(MemoryDisk::default());
    let mut service = Service::open(disk.clone(), clock(0, "startup")).unwrap();
    let payload;
    let now;
    if kind == "begin-session-edit" || kind == "finish-session-edit" {
        exec(&mut service, "a", "start", json!({"name":"work"}), 0);
        payload = if kind == "begin-session-edit" {
            now = 10;
            json!({"activityId":"a","sessionId":"a:0","progress":"draft"})
        } else {
            exec(
                &mut service,
                "edit-token",
                "begin-session-edit",
                json!({"activityId":"a","sessionId":"a:0","progress":"draft"}),
                10,
            );
            now = 20;
            json!({"activityId":"a","sessionId":"a:0","editToken":"edit:edit-token","cancel":false,"progress":"saved"})
        };
    } else {
        exec(
            &mut service,
            "a",
            "add-session",
            json!({"name":"first","startTime":t(0),"endTime":t(10),"progress":"one"}),
            30,
        );
        exec(
            &mut service,
            "b",
            "add-session",
            json!({"name":"second","startTime":t(20),"endTime":t(30),"progress":"two"}),
            30,
        );
        now = 40;
        payload = match kind {
            "merge-sessions" => {
                json!({"first":{"activityId":"a","sessionId":"a:0"},"second":{"activityId":"b","sessionId":"b:0"},"targetActivityId":"a"})
            }
            "gap" => {
                json!({"first":{"activityId":"a","sessionId":"a:0"},"gap":{"startTime":t(10),"endTime":t(20)},"targetActivityId":"a"})
            }
            "split-session" => json!({"activityId":"a","sessionId":"a:0","atTime":t(5)}),
            _ => panic!("unknown scenario"),
        };
    }
    let request = request(
        "operation",
        service.snapshot().tracking.revision,
        if kind == "gap" {
            "merge-sessions"
        } else {
            kind
        },
        payload,
    );
    disk.inner.lock().unwrap().writes.clear();
    (disk, service, request, now)
}
const KINDS: [&str; 5] = [
    "merge-sessions",
    "gap",
    "split-session",
    "begin-session-edit",
    "finish-session-edit",
];
#[test]
fn all_timeline_commands_use_one_tracking_commit_and_exact_replay_with_conflicts() {
    for kind in KINDS {
        let (disk, mut service, request, now) = setup(kind);
        let revision = service.snapshot().tracking.revision;
        let result = service
            .execute(request.clone(), clock(now, "operation"))
            .unwrap();
        assert_eq!(result.tracking.revision, revision + 1);
        assert_eq!(
            disk.inner
                .lock()
                .unwrap()
                .writes
                .iter()
                .filter(|k| k.as_str() == "activity-log.json")
                .count(),
            1
        );
        let duplicate = service
            .execute(request.clone(), clock(now + 5, "ignored"))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::to_value(duplicate).unwrap()
        );
        let mut stale = request.clone();
        stale.request_id = "stale".into();
        assert_eq!(
            service
                .execute(stale, clock(now + 5, "ignored"))
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
        let mut collision = request;
        collision.expected_revision = revision + 1;
        assert_eq!(
            service
                .execute(collision, clock(now + 5, "ignored"))
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
        assert_eq!(service.snapshot().tracking, result.tracking);
        assert_eq!(
            disk.inner
                .lock()
                .unwrap()
                .writes
                .iter()
                .filter(|k| k.as_str() == "activity-log.json")
                .count(),
            1
        );
    }
}
#[test]
fn failure_before_or_after_each_tracking_write_never_publishes_partial_state_or_caches_success() {
    for kind in KINDS {
        for key in ["activity-log.json.bak", "activity-log.json"] {
            for after in [false, true] {
                let (disk, mut service, request, now) = setup(kind);
                let original = service.snapshot().tracking;
                let expected = original
                    .apply(
                        match &request.operation {
                            UserCommand::Tracking(c) => c,
                            _ => unreachable!(),
                        },
                        original.revision,
                        tracking::Context {
                            now: clock(now, "operation").now,
                            new_id: "operation",
                        },
                    )
                    .unwrap();
                disk.inner.lock().unwrap().failure = Some((key.into(), after));
                let error = service
                    .execute(request.clone(), clock(now, "operation"))
                    .unwrap_err();
                assert_eq!(
                    error.code,
                    ErrorCode::StorageFailure,
                    "{kind} {key} after={after}"
                );
                assert_eq!(service.snapshot().tracking, original);
                if key == "activity-log.json" && after {
                    assert!(service.snapshot().storage_blocked);
                    assert_eq!(
                        service
                            .execute(request.clone(), clock(now, "operation"))
                            .unwrap_err()
                            .code,
                        ErrorCode::StorageFailure
                    );
                    let restored = Service::open(disk.clone(), clock(now, "restart")).unwrap();
                    let expected = expected
                        .recover_interrupted(clock(now, "restart").now)
                        .unwrap();
                    assert_eq!(restored.snapshot().tracking, expected);
                } else {
                    assert!(!service.snapshot().storage_blocked);
                    let result = service
                        .execute(request.clone(), clock(now, "operation"))
                        .unwrap();
                    assert_eq!(result.tracking, expected);
                    assert_eq!(
                        service
                            .execute(request, clock(now + 1, "ignored"))
                            .unwrap()
                            .tracking,
                        expected
                    );
                }
            }
        }
    }
}
#[test]
fn failed_cancel_keeps_edit_open_and_same_request_can_retry_without_applying_fields() {
    let (disk, mut service, mut request, now) = setup("finish-session-edit");
    if let UserCommand::Tracking(Command::FinishSessionEdit {
        cancel, progress, ..
    }) = &mut request.operation
    {
        *cancel = true;
        *progress = Some("must not apply".into());
    }
    let before = service.snapshot().tracking;
    disk.inner.lock().unwrap().failure = Some(("activity-log.json".into(), false));
    assert_eq!(
        service
            .execute(request.clone(), clock(now, "cancel"))
            .unwrap_err()
            .code,
        ErrorCode::StorageFailure
    );
    assert_eq!(service.snapshot().tracking, before);
    assert_eq!(
        service.snapshot().tracking.activities[0]
            .pause_reason
            .as_deref(),
        Some("edit:edit-token")
    );
    let result = service.execute(request, clock(now, "cancel")).unwrap();
    assert_eq!(result.tracking.activities[0].status, Status::Running);
    let rows = result.tracking.activities[0].sessions.as_ref().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].progress, "draft");
}
#[test]
fn invalid_edits_and_wrong_tokens_write_nothing_and_the_original_token_remains_retryable() {
    let (disk, mut service, mut request, now) = setup("finish-session-edit");
    let before = service.snapshot().tracking;
    if let UserCommand::Tracking(Command::FinishSessionEdit { end_time, .. }) =
        &mut request.operation
    {
        *end_time = Some(t(now + 10));
    }
    assert_eq!(
        service
            .execute(request.clone(), clock(now, "save"))
            .unwrap_err()
            .code,
        ErrorCode::Validation
    );
    assert_eq!(service.snapshot().tracking, before);
    assert!(disk.inner.lock().unwrap().writes.is_empty());
    if let UserCommand::Tracking(Command::FinishSessionEdit {
        end_time,
        edit_token,
        ..
    }) = &mut request.operation
    {
        *end_time = None;
        *edit_token = "edit:foreign".into();
    }
    assert_eq!(
        service
            .execute(request.clone(), clock(now, "save"))
            .unwrap_err()
            .code,
        ErrorCode::Validation
    );
    assert!(disk.inner.lock().unwrap().writes.is_empty());
    if let UserCommand::Tracking(Command::FinishSessionEdit { edit_token, .. }) =
        &mut request.operation
    {
        *edit_token = "edit:edit-token".into();
    }
    assert!(service.execute(request, clock(now, "save")).is_ok());
}
#[test]
fn an_external_storage_edit_blocks_timeline_mutations_without_overwriting_it() {
    for kind in KINDS {
        let (disk, mut service, request, now) = setup(kind);
        let before = service.snapshot().tracking;
        disk.inner
            .lock()
            .unwrap()
            .files
            .insert("activity-log.json".into(), b"external bytes".to_vec());
        assert_eq!(
            service
                .execute(request, clock(now, "operation"))
                .unwrap_err()
                .code,
            ErrorCode::StorageFailure
        );
        assert_eq!(
            disk.read("activity-log.json").unwrap().unwrap(),
            b"external bytes"
        );
        assert_eq!(service.snapshot().tracking, before);
        assert!(service.snapshot().storage_blocked);
    }
}
fn effort(service: &Service, now: i64) -> i64 {
    service
        .growth_view(
            growth::DayBounds {
                day: "2026-11-14",
                start: 0,
                end: 86_400_000,
            },
            clock(now, "unused").now,
        )
        .unwrap()
        .invested_ms
}
#[test]
fn growth_projection_invalidates_on_timeline_revisions_without_duration_loss_or_double_count() {
    let (_, mut service, request, now) = setup("merge-sessions");
    let before = effort(&service, now);
    assert_eq!(before, 20 * 60_000);
    service.execute(request, clock(now, "merge")).unwrap();
    assert_eq!(effort(&service, now), before);
    exec(
        &mut service,
        "split",
        "split-session",
        json!({"activityId":"a","sessionId":"a:0","atTime":t(5)}),
        now,
    );
    assert_eq!(effort(&service, now), before);
    exec(
        &mut service,
        "gap",
        "merge-sessions",
        json!({"first":{"activityId":"a","sessionId":"a:1"},"gap":{"startTime":t(30),"endTime":t(35)},"targetActivityId":"a"}),
        now,
    );
    assert_eq!(effort(&service, now), before + 5 * 60_000);
    let (_, mut service, request, now) = setup("begin-session-edit");
    assert_eq!(effort(&service, 10), 10 * 60_000);
    service.execute(request, clock(now, "token")).unwrap();
    assert_eq!(effort(&service, 20), 10 * 60_000);
    exec(
        &mut service,
        "finish",
        "finish-session-edit",
        json!({"activityId":"a","sessionId":"a:0","editToken":"edit:token","cancel":false}),
        20,
    );
    assert_eq!(effort(&service, 30), 20 * 60_000);
}
#[test]
fn edit_token_persists_across_restart_and_another_started_matter_is_not_stolen() {
    for cancel in [true, false] {
        let (disk, service, _, _) = setup("finish-session-edit");
        drop(service);
        let mut restored = Service::open(disk, clock(12, "restart")).unwrap();
        exec(&mut restored, "b", "start", json!({"name":"other"}), 15);
        let result = exec(
            &mut restored,
            "save",
            "finish-session-edit",
            json!({"activityId":"a","sessionId":"a:0","editToken":"edit:edit-token","cancel":cancel,"progress":"saved"}),
            20,
        );
        assert_eq!(result.tracking.current().unwrap().id, "b");
        assert_eq!(result.tracking.activities[0].status, Status::Paused);
        assert_eq!(
            result.tracking.activities[0].pause_reason.as_deref(),
            Some("manual")
        );
        assert_eq!(effort(&restored, 30), 25 * 60_000);
    }
}
