use catch_core::todos::{Command, Context, Document};
use serde::Deserialize;
use serde_json::Value;
#[derive(Deserialize)]
struct Fixture {
    steps: Vec<Step>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Step {
    command: Value,
    now: i64,
    local_day: String,
    new_id: String,
    rejected: bool,
    expected: Value,
}
#[test]
fn todo_lifecycle_and_rejections_match_the_existing_application() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("fixtures/todos-parity.json")).unwrap();
    let mut state = Document::default();
    for (index, step) in fixture.steps.into_iter().enumerate() {
        let command: Command = serde_json::from_value(step.command).unwrap();
        let result = state.apply(
            &command,
            state.revision,
            Context {
                now: step.now,
                local_day: &step.local_day,
                new_id: &step.new_id,
            },
        );
        assert_eq!(
            result.is_err(),
            step.rejected,
            "Step {}: {:?}",
            index + 1,
            result.as_ref().err()
        );
        if let Ok(next) = result {
            state = next;
        }
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            step.expected,
            "Step {}",
            index + 1
        );
    }
}

#[test]
fn restoring_an_archived_todo_preserves_its_identity_resources_and_extension_fields() {
    let row = serde_json::json!({
        "id":"archived", "text":"阅读", "notes":"保留原备注", "done":true,
        "urgent":true, "deadline":"2026-11-16", "completedAt":"2026-11-15",
        "resources":[{"id":"reference","kind":"web","title":"参考","target":"https://example.com"}],
        "custom":{"preserve":true}
    });
    for duplicated in [false, true] {
        let state: Document = serde_json::from_value(serde_json::json!({
            "todos": if duplicated { vec![row.clone()] } else { vec![] },
            "archive":[row.clone()], "revision":3
        }))
        .unwrap();
        let restored = state
            .apply(
                &Command::RestoreArchive {
                    id: "archived".into(),
                },
                3,
                Context {
                    now: 0,
                    local_day: "2026-11-16",
                    new_id: "unused",
                },
            )
            .unwrap();
        assert!(restored.archive.is_empty());
        assert_eq!(restored.todos.len(), 1);
        let mut expected = row.clone();
        expected["done"] = false.into();
        expected["completedAt"] = Value::Null;
        assert_eq!(serde_json::to_value(&restored.todos[0]).unwrap(), expected);
        assert!(restored
            .apply(
                &Command::RestoreArchive {
                    id: "archived".into()
                },
                restored.revision,
                Context {
                    now: 0,
                    local_day: "2026-11-16",
                    new_id: "unused"
                }
            )
            .is_err());
        assert_eq!(state.archive.len(), 1);
    }
}
