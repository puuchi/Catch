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
            resources: vec![],
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
                    resources: vec![],
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
                    resources: vec![],
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

#[test]
fn competing_windows_cannot_commit_two_plants_and_restart_keeps_the_winner() {
    use catch_core::growth::{Command, Plant};
    let (dir, service) = fixture();
    let service = Arc::new(Mutex::new(service));
    let threads: Vec<_> = [
        Plant::Corn,
        Plant::Strawberry,
        Plant::Sunflower,
        Plant::Tulip,
        Plant::Sprout,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, plant)| {
        let service = service.clone();
        std::thread::spawn(move || {
            service
                .lock()
                .unwrap()
                .execute(
                    Request {
                        request_id: format!("plant-{index}"),
                        expected_revision: 0,
                        operation: UserCommand::Growth(Command::Choose { plant }),
                    },
                    context("unused"),
                )
                .is_ok()
        })
    })
    .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|ok| *ok)
            .count(),
        1
    );
    let selected = service.lock().unwrap().snapshot().growth.days["2026-11-14"].plant;
    drop(service);
    let mut restored = Service::open(
        Arc::new(FileDisk::new(dir.path()).unwrap()),
        context("restarted"),
    )
    .unwrap();
    assert_eq!(
        restored.snapshot().growth.days["2026-11-14"].plant,
        selected
    );
    restored
        .execute(
            Request {
                request_id: "goal".into(),
                expected_revision: 1,
                operation: UserCommand::Growth(Command::SetGoal { minutes: None }),
            },
            context("unused"),
        )
        .unwrap();
    let another = if selected == Plant::Corn {
        Plant::Strawberry
    } else {
        Plant::Corn
    };
    assert!(restored
        .execute(
            Request {
                request_id: "replace".into(),
                expected_revision: 2,
                operation: UserCommand::Growth(Command::Choose { plant: another })
            },
            context("unused")
        )
        .is_err());
    restored
        .execute(
            Request {
                request_id: "tomorrow".into(),
                expected_revision: 2,
                operation: UserCommand::Growth(Command::Choose { plant: another }),
            },
            Context {
                now: 86_400_000,
                local_day: "2026-11-15",
                new_id: "unused",
            },
        )
        .unwrap();
    assert_eq!(
        restored.snapshot().growth.days["2026-11-14"].plant,
        selected
    );
    assert_eq!(restored.snapshot().growth.days["2026-11-15"].plant, another);
}

#[test]
fn daily_effort_uses_effective_intervals_and_achievement_is_durable_once() {
    use catch_core::growth::{Command, DayBounds, Plant};
    let (_dir, mut service) = fixture();
    service
        .execute(
            Request {
                request_id: "plant".into(),
                expected_revision: 0,
                operation: UserCommand::Growth(Command::Choose { plant: Plant::Corn }),
            },
            context("unused"),
        )
        .unwrap();
    service.execute(start("start", 0), context("a")).unwrap();
    service
        .execute(
            Request {
                request_id: "pause".into(),
                expected_revision: 1,
                operation: UserCommand::Tracking(tracking::Command::Pause {
                    reason: "manual".into(),
                    cutoff: None,
                }),
            },
            Context {
                now: 3_600_000,
                ..context("unused")
            },
        )
        .unwrap();
    let bounds = || DayBounds {
        day: "2026-11-14",
        start: 0,
        end: 86_400_000,
    };
    assert!(service.observe_growth(bounds(), 4_000_000).unwrap());
    assert!(!service.observe_growth(bounds(), 5_000_000).unwrap());
    let view = service.growth_view(bounds(), 10_000_000).unwrap();
    assert_eq!(view.invested_ms, 3_600_000);
    assert_eq!(view.stage, 4);
    assert!(view.celebrated);
    assert_eq!(
        service
            .growth_view(
                DayBounds {
                    day: "2026-11-15",
                    start: 86_400_000,
                    end: 172_800_000
                },
                90_000_000
            )
            .unwrap()
            .invested_ms,
        0
    );
}

#[test]
fn matters_keep_independent_resource_snapshots_after_todo_relocation_and_deletion() {
    use catch_core::resources::{Command, Kind, Owner};
    let (_dir, mut service) = fixture();
    service
        .execute(
            Request {
                request_id: "todo".into(),
                expected_revision: 0,
                operation: UserCommand::Todos(todos::Command::Create(todos::Fields {
                    text: "阅读材料".into(),
                    notes: String::new(),
                    urgent: false,
                    deadline: None,
                })),
            },
            context("todo-1"),
        )
        .unwrap();
    service
        .execute(
            Request {
                request_id: "attach".into(),
                expected_revision: 1,
                operation: UserCommand::Resources(Command::Attach {
                    owner: Owner::Todo {
                        id: "todo-1".into(),
                    },
                    kind: Kind::File,
                    target: r"D:\资料\阅读.pdf".into(),
                    title: None,
                }),
            },
            context("resource-1"),
        )
        .unwrap();
    service
        .execute(
            Request {
                request_id: "start-linked".into(),
                expected_revision: 0,
                operation: UserCommand::Tracking(tracking::Command::Start {
                    name: "stale".into(),
                    todo_id: Some("todo-1".into()),
                    resources: vec![],
                }),
            },
            context("matter-1"),
        )
        .unwrap();
    service
        .execute(
            Request {
                request_id: "relocate".into(),
                expected_revision: 2,
                operation: UserCommand::Resources(Command::Relocate {
                    owner: Owner::Todo {
                        id: "todo-1".into(),
                    },
                    resource_id: "resource-1".into(),
                    target: r"D:\资料\新版.pdf".into(),
                }),
            },
            context("unused"),
        )
        .unwrap();
    service
        .execute(
            Request {
                request_id: "trash".into(),
                expected_revision: 3,
                operation: UserCommand::Todos(todos::Command::Trash {
                    id: "todo-1".into(),
                }),
            },
            context("unused"),
        )
        .unwrap();
    service
        .execute(
            Request {
                request_id: "delete".into(),
                expected_revision: 4,
                operation: UserCommand::Todos(todos::Command::Delete {
                    id: "todo-1".into(),
                }),
            },
            context("unused"),
        )
        .unwrap();
    let resource = service
        .resource_for_open(
            &Owner::Matter {
                id: "matter-1".into(),
            },
            "resource-1",
        )
        .unwrap();
    assert_eq!(resource.target, r"D:\资料\阅读.pdf");
    assert!(service
        .resource_for_open(
            &Owner::Todo {
                id: "todo-1".into()
            },
            "resource-1"
        )
        .is_err());
    service
        .execute(
            Request {
                request_id: "remove-link".into(),
                expected_revision: 1,
                operation: UserCommand::Resources(Command::Remove {
                    owner: Owner::Matter {
                        id: "matter-1".into(),
                    },
                    resource_id: "resource-1".into(),
                }),
            },
            context("unused"),
        )
        .unwrap();
    assert!(service.snapshot().tracking.activities[0]
        .resources
        .is_empty());
}

#[test]
fn recurring_requests_check_both_versions_and_startup_refreshes_only_one_occurrence() {
    use catch_app::RecurrenceRequest;
    use catch_core::recurrence::{Command, Schedule};
    let (dir, mut service) = fixture();
    let create = Request {
        request_id: "recurring".into(),
        expected_revision: 0,
        operation: UserCommand::Recurrence(RecurrenceRequest {
            command: Command::Create {
                fields: todos::Fields {
                    text: "每日阅读".into(),
                    notes: String::new(),
                    urgent: false,
                    deadline: None,
                },
                resources: vec![],
                schedule: Schedule::Daily,
                starts_on: "2026-11-14".into(),
            },
            expected_todos_revision: 0,
        }),
    };
    service.execute(create.clone(), context("rule-1")).unwrap();
    service
        .execute(create, context("should-not-create"))
        .unwrap();
    assert_eq!(service.snapshot().todos.todos.len(), 1);
    let error = service
        .execute(
            Request {
                request_id: "stale-edit".into(),
                expected_revision: service.snapshot().recurrences.revision,
                operation: UserCommand::Recurrence(RecurrenceRequest {
                    command: Command::Stop {
                        id: "rule-1".into(),
                    },
                    expected_todos_revision: 0,
                }),
            },
            context("unused"),
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    let state = service.snapshot();
    let id = state.todos.todos[0].id.clone();
    service
        .execute(
            Request {
                request_id: "finish".into(),
                expected_revision: state.todos.revision,
                operation: UserCommand::Todos(todos::Command::Complete { id, done: true }),
            },
            context("unused"),
        )
        .unwrap();
    drop(service);
    let disk = Arc::new(FileDisk::new(dir.path()).unwrap());
    let next = Service::open(
        disk.clone(),
        Context {
            now: 86_400_000,
            local_day: "2026-11-15",
            new_id: "startup-2",
        },
    )
    .unwrap();
    assert_eq!(next.snapshot().todos.todos.len(), 2);
    assert_eq!(
        next.snapshot()
            .todos
            .todos
            .iter()
            .filter(|todo| !todo.done)
            .count(),
        1
    );
    drop(next);
    let later = Service::open(
        disk,
        Context {
            now: 3_600_000_000,
            local_day: "2026-12-15",
            new_id: "startup-3",
        },
    )
    .unwrap();
    assert_eq!(later.snapshot().todos.todos.len(), 2);
}
