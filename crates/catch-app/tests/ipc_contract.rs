use catch_app::{Error, ErrorCode, FinishRequest, Reply, Request, Snapshot, UserCommand};
use catch_core::{growth, recurrence, resources, todos, tracking};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};

fn snapshot() -> Snapshot {
    let references = vec![
        resources::Resource::new("file", resources::Kind::File, r"D:\资料\阅读.pdf", None).unwrap(),
        resources::Resource::new("folder", resources::Kind::Folder, r"D:\资料", Some("目录"))
            .unwrap(),
        resources::Resource::new(
            "web",
            resources::Kind::Web,
            "https://example.com",
            Some("参考页面"),
        )
        .unwrap(),
    ];
    let mut tracking = tracking::Document::default();
    for (index, id) in ["completed", "paused", "running"].iter().enumerate() {
        tracking = tracking
            .apply(
                &tracking::Command::Start {
                    name: format!("事项{index}"),
                    todo_id: Some("todo".into()),
                    resources: references.clone(),
                },
                tracking.revision,
                tracking::Context {
                    now: index as i64 * 1000,
                    new_id: id,
                },
            )
            .unwrap();
    }
    tracking = tracking
        .finish(
            "completed",
            Some("completed:0"),
            Some("最终进展"),
            tracking.revision,
            tracking::Context {
                now: 3000,
                new_id: "unused",
            },
        )
        .unwrap();
    let archived = tracking.activities[0].clone();
    tracking.trash = serde_json::from_value(json!([
        {"id":"trash-matter","kind":"matter","deletedAt":"1970-01-01T00:00:04.000Z","activity":archived},
        {"id":"trash-session","kind":"session","deletedAt":"1970-01-01T00:00:04.000Z","activity":archived,"session":archived.sessions.as_ref().unwrap()[0]}
    ])).unwrap();
    let mut todos: todos::Document = serde_json::from_value(json!({
        "revision":3,
        "todos":[{"id":"todo","text":"阅读","notes":"备注","urgent":true,"done":false,"deadline":"2026-11-18","completedAt":null,
            "recurrence":{"templateId":"daily","scheduledOn":"2026-11-18"},"resources":references},
            {"id":"deleted","text":"删除的待办","done":false,"deletedAt":"2026-11-18T00:00:00Z"}],
        "archive":[{"id":"archived","text":"归档待办","done":true,"completedAt":"2026-11-17"}]
    })).unwrap();
    todos.archive[0].resources = references;
    let schedules = [
        ("daily", json!({"kind":"daily"})),
        ("weekly", json!({"kind":"weekly","weekdays":[1,3,5]})),
        ("every", json!({"kind":"every","days":2})),
        ("monthly", json!({"kind":"monthly","day":31})),
    ];
    let templates: Vec<Value> = schedules.into_iter().map(|(id, schedule)| json!({
        "id":id,"fields":{"text":"重复待办","notes":"","urgent":false,"deadline":null},"resources":[],
        "schedule":schedule,"startsOn":"2026-11-18","nextOn":"2026-11-19","lastEmitted":null,"stopped":false
    })).collect();
    let recurrences: recurrence::Document =
        serde_json::from_value(json!({"version":1,"revision":4,"templates":templates})).unwrap();
    let mut days = BTreeMap::new();
    for (index, plant) in [
        growth::Plant::Tulip,
        growth::Plant::Sunflower,
        growth::Plant::Corn,
        growth::Plant::Strawberry,
        growth::Plant::Sprout,
    ]
    .into_iter()
    .enumerate()
    {
        days.insert(
            format!("2026-11-{:02}", 13 + index),
            growth::Day {
                plant,
                chosen_at: "2026-11-13T00:00:00Z".into(),
                celebrated_at: (index == 0).then(|| "2026-11-13T01:00:00Z".into()),
                goal_on_celebration: (index == 0).then_some(60),
            },
        );
    }
    Snapshot {
        tracking,
        todos,
        recurrences,
        growth: growth::Document {
            days,
            ..growth::Document::default()
        },
        tracking_notice: None,
        storage_blocked: false,
    }
}

#[test]
fn serde_ipc_contract_matches_the_frontend_fixture() {
    let snapshot = snapshot();
    let finish = Request {
        request_id: "finish".into(),
        expected_revision: 4,
        operation: UserCommand::Finish(FinishRequest {
            activity_id: "paused".into(),
            session_id: Some("paused:0".into()),
            progress: Some("已整理".into()),
            complete_todo: true,
            expected_todos_revision: 3,
        }),
    };
    let default_finish: Request = serde_json::from_value(
        json!({"requestId":"default-finish","expectedRevision":4,"operation":{
            "group":"finish","command":{"activityId":"paused","expectedTodosRevision":3}
        }}),
    )
    .unwrap();
    let errors = [
        ErrorCode::Validation,
        ErrorCode::Conflict,
        ErrorCode::StorageFailure,
    ]
    .into_iter()
    .map(|code| Error {
        code,
        message: "示例错误".into(),
    })
    .collect::<Vec<_>>();
    let view = growth::View {
        day: "2026-11-18".into(),
        plant: growth::Plant::Tulip,
        selected: true,
        invested_ms: 60000,
        goal_minutes: Some(60),
        stage: 1,
        reached: false,
        celebrated: false,
    };
    let mut blocked = snapshot.clone();
    blocked.storage_blocked = true;
    blocked.tracking_notice = Some("恢复提示".into());
    blocked.growth.goal_minutes = None;
    let mut reply = Reply::from_result(Ok(snapshot.clone()), snapshot);
    reply.growth = Some(view);
    reply.now = Some(60000);
    let failure = Reply::from_result(
        Err(Error {
            code: ErrorCode::StorageFailure,
            message: "恢复提示".into(),
        }),
        blocked,
    );
    let actual = json!({ "reply":reply, "failureReply":failure, "finishRequest":finish, "defaultFinishRequest":default_finish, "errors":errors });
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../frontend/test/fixtures/rust-ipc.json");
    if std::env::var("UPDATE_IPC_CONTRACT").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
        )
        .unwrap();
    }
    let expected: Value = serde_json::from_slice(
        &std::fs::read(&path).expect("generate rust-ipc.json with UPDATE_IPC_CONTRACT=1"),
    )
    .unwrap();
    assert_eq!(
        expected, actual,
        "Rust IPC changed. Regenerate the fixture, then run the frontend contract type check."
    );
}
