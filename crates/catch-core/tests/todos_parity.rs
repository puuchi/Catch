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
