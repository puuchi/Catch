use catch_core::tracking::{Command, Context, Document};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Fixture {
    scenarios: Vec<Scenario>,
}
#[derive(Deserialize)]
struct Scenario {
    name: String,
    initial: Document,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Step {
    command: Value,
    now: i64,
    new_id: String,
    rejected: bool,
    expected: Value,
}

#[test]
fn existing_electron_rules_and_rust_produce_identical_records() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("fixtures/tracking-parity.json")).unwrap();
    for scenario in fixture.scenarios {
        let mut state = scenario.initial;
        for (index, step) in scenario.steps.into_iter().enumerate() {
            let command: Command =
                serde_json::from_value(step.command.clone()).unwrap_or_else(|error| {
                    panic!(
                        "{} step {} {:?}: {error}",
                        scenario.name,
                        index + 1,
                        step.command
                    )
                });
            let before = state.clone();
            let result = state.apply(
                &command,
                state.revision,
                Context {
                    now: step.now,
                    new_id: &step.new_id,
                },
            );
            assert_eq!(
                result.is_err(),
                step.rejected,
                "{} step {} {:?}: {:?}",
                scenario.name,
                index + 1,
                command,
                result.as_ref().err()
            );
            if let Ok(next) = result {
                state = next;
            } else {
                assert_eq!(before, state);
            }
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                step.expected,
                "{} step {} {:?}",
                scenario.name,
                index + 1,
                command
            );
        }
    }
}

#[test]
fn stale_revision_leaves_original_unchanged() {
    let state = Document::default();
    let before = state.clone();
    assert!(state
        .apply(
            &Command::Start {
                name: "A".into(),
                todo_id: None,
                resources: vec![],
            },
            5,
            Context {
                now: 0,
                new_id: "a"
            }
        )
        .is_err());
    assert_eq!(state, before);
}

#[test]
fn legacy_sessions_and_extensions_survive_migration_and_recovery() {
    let input = serde_json::json!({"version":1,"checkpointAt":"2026-11-13T23:31:00.000Z","activities":[{
        "id":"legacy","name":"阅读","startTime":"2026-11-13T23:30:00.000Z","endTime":null,"status":"running",
        "segments":[{"start":"2026-11-13T23:30:00.000Z","end":null}],"todoId":"todo-old",
        "createdAt":"2026-11-13T23:30:00.000Z","updatedAt":"2026-11-13T23:30:00.000Z","pauseReason":null,
        "customLinks":[{"id":"file-1","path":"D:/资料/阅读.pdf"}]}],"custom":{"preserve":true}});
    let state: Document = serde_json::from_value(input).unwrap();
    let migrated = state.migrated().unwrap();
    let recovered = migrated
        .recover_interrupted(catch_core::tracking::timestamp("2026-11-14T10:00:00Z").unwrap())
        .unwrap();
    let matter = &recovered.activities[0];
    assert_eq!(matter.sessions.as_ref().unwrap()[0].id, "legacy:0");
    assert_eq!(matter.duration(i64::MAX, None).unwrap(), 60_000);
    assert_eq!(matter.todo_id.as_deref(), Some("todo-old"));
    assert_eq!(matter.extra["customLinks"][0]["id"], "file-1");
    assert_eq!(recovered.extra["custom"]["preserve"], true);
    assert!(state.activities[0].sessions.is_none());
}

#[test]
fn future_schema_and_inconsistent_flattened_intervals_are_rejected() {
    let state = Document {
        version: 3,
        ..Document::default()
    };
    assert!(state.migrated().is_err());
    let fixture: Fixture =
        serde_json::from_str(include_str!("fixtures/tracking-parity.json")).unwrap();
    let mut state: Document =
        serde_json::from_value(fixture.scenarios[0].steps[0].expected.clone()).unwrap();
    state.activities[0].segments.clear();
    assert!(state.validate().is_err());
}

#[test]
fn day_clipping_uses_boundaries_without_a_fixed_day_length() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("fixtures/tracking-parity.json")).unwrap();
    let state: Document =
        serde_json::from_value(fixture.scenarios[0].steps[1].expected.clone()).unwrap();
    let start = catch_core::tracking::timestamp(&state.activities[0].start_time).unwrap();
    assert_eq!(
        state.activities[0]
            .duration(start + 120_000, Some((start + 30_000, start + 90_000)))
            .unwrap(),
        30_000
    );
}
