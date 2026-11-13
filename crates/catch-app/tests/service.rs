use catch_app::{Context, ErrorCode, Request, Service, UserCommand};
use catch_core::{todos, tracking};
use catch_storage::FileDisk;
use std::sync::{Arc, Mutex};
fn context(id: &str) -> Context<'_> {
    Context {
        now: 0,
        local_day: "2026-11-14",
        new_id: id,
    }
}
fn fixture() -> (tempfile::TempDir, Service) {
    let dir = tempfile::tempdir().unwrap();
    let service = Service::open(
        Arc::new(FileDisk::new(dir.path()).unwrap()),
        context("startup"),
    )
    .unwrap();
    (dir, service)
}
fn start(request_id: &str, revision: u64) -> Request {
    Request {
        request_id: request_id.into(),
        expected_revision: revision,
        operation: UserCommand::Tracking(tracking::Command::Start {
            name: "阅读".into(),
            todo_id: None,
        }),
    }
}
#[test]
fn replay_returns_the_original_result_and_collision_or_stale_commands_do_not_write() {
    let (_dir, mut service) = fixture();
    let request = start("once", 0);
    let original = service.execute(request.clone(), context("a")).unwrap();
    let replay = service.execute(request, context("duplicate")).unwrap();
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    assert_eq!(service.snapshot().tracking.activities.len(), 1);
    assert_eq!(
        service
            .execute(start("once", 1), context("b"))
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        service
            .execute(start("stale", 0), context("b"))
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(service.snapshot().tracking.activities.len(), 1);
}
#[test]
fn starting_from_a_todo_uses_authoritative_text_and_rejects_completed_or_deleted_todos() {
    let (_dir, mut service) = fixture();
    let fields = todos::Fields {
        text: "整理材料".into(),
        notes: String::new(),
        urgent: false,
        deadline: None,
    };
    service
        .execute(
            Request {
                request_id: "todo".into(),
                expected_revision: 0,
                operation: UserCommand::Todos(todos::Command::Create(fields)),
            },
            context("todo-1"),
        )
        .unwrap();
    service
        .execute(
            Request {
                request_id: "start".into(),
                expected_revision: 0,
                operation: UserCommand::Tracking(tracking::Command::Start {
                    name: "stale text".into(),
                    todo_id: Some("todo-1".into()),
                }),
            },
            context("matter-1"),
        )
        .unwrap();
    assert_eq!(service.snapshot().tracking.activities[0].name, "整理材料");
    service
        .execute(
            Request {
                request_id: "done".into(),
                expected_revision: 1,
                operation: UserCommand::Todos(todos::Command::Complete {
                    id: "todo-1".into(),
                    done: true,
                }),
            },
            context("unused"),
        )
        .unwrap();
    let error = service
        .execute(
            Request {
                request_id: "start-again".into(),
                expected_revision: 1,
                operation: UserCommand::Tracking(tracking::Command::Start {
                    name: "bad".into(),
                    todo_id: Some("todo-1".into()),
                }),
            },
            context("matter-2"),
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Validation);
    assert_eq!(service.snapshot().tracking.activities.len(), 1);
}
#[test]
fn simultaneous_requests_are_serialized_and_only_one_wins_a_revision() {
    let (_dir, service) = fixture();
    let service = Arc::new(Mutex::new(service));
    let threads: Vec<_> = (0..8)
        .map(|index| {
            let service = service.clone();
            std::thread::spawn(move || {
                service
                    .lock()
                    .unwrap()
                    .execute(
                        start(&format!("request-{index}"), 0),
                        context(&format!("id-{index}")),
                    )
                    .is_ok()
            })
        })
        .collect();
    let successes = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .filter(|ok| *ok)
        .count();
    assert_eq!(successes, 1);
    assert_eq!(
        service.lock().unwrap().snapshot().tracking.activities.len(),
        1
    );
}
#[test]
fn renderer_cannot_supply_system_cutoffs_or_invoke_internal_checkpoint() {
    let (_dir, mut service) = fixture();
    for command in [
        tracking::Command::Checkpoint,
        tracking::Command::Pause {
            reason: "sleep".into(),
            cutoff: Some(-1000),
        },
    ] {
        let result = service.execute(
            Request {
                request_id: "invalid".into(),
                expected_revision: 0,
                operation: UserCommand::Tracking(command),
            },
            context("unused"),
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::Validation);
    }
}
