use catch_core::growth::{Command, Context, Document, Plant};
fn context(day: &str) -> Context<'_> {
    Context {
        now: 0,
        local_day: day,
    }
}
#[test]
fn daily_choice_is_fixed_across_goals_restarts_and_earlier_dates() {
    let state = Document::default();
    assert!(state.days.is_empty());
    let state = state
        .apply(
            &Command::Choose { plant: Plant::Corn },
            0,
            context("2026-11-14"),
        )
        .unwrap();
    assert!(state
        .apply(
            &Command::Choose {
                plant: Plant::Strawberry
            },
            1,
            context("2026-11-14")
        )
        .is_err());
    let state = state
        .apply(
            &Command::SetGoal { minutes: Some(30) },
            1,
            context("2026-11-14"),
        )
        .unwrap();
    let serialized = serde_json::to_vec(&state).unwrap();
    let state: Document = serde_json::from_slice(&serialized).unwrap();
    assert!(state
        .apply(
            &Command::Choose {
                plant: Plant::Tulip
            },
            2,
            context("2026-11-14")
        )
        .is_err());
    let later = state
        .apply(
            &Command::Choose {
                plant: Plant::Strawberry,
            },
            2,
            context("2026-11-15"),
        )
        .unwrap();
    assert_eq!(later.days["2026-11-14"].plant, Plant::Corn);
    assert_eq!(later.days["2026-11-15"].plant, Plant::Strawberry);
    assert!(later
        .apply(
            &Command::Choose {
                plant: Plant::Sunflower
            },
            3,
            context("2026-11-14")
        )
        .is_err());
}
#[test]
fn preview_and_effort_do_not_choose_a_plant_or_lock_tomorrow() {
    let state = Document::default();
    for minutes in [0, 6, 18, 36, 60] {
        let view = state.view("2026-11-14", minutes * 60_000).unwrap();
        assert!(!view.selected);
    }
    assert_eq!(
        state.observe(context("2026-11-14"), 3_600_000).unwrap(),
        state
    );
    let state = state
        .apply(
            &Command::Choose {
                plant: Plant::Tulip,
            },
            0,
            context("2026-11-14"),
        )
        .unwrap();
    let next = state.view("2026-11-15", 0).unwrap();
    assert_eq!(next.plant, Plant::Tulip);
    assert!(!next.selected);
    assert_eq!(next.stage, 0);
}
#[test]
fn five_growth_stages_have_distinct_exact_boundaries() {
    let state = Document::default();
    for (ms, stage) in [
        (0, 0),
        (359_999, 0),
        (360_000, 1),
        (1_079_999, 1),
        (1_080_000, 2),
        (2_159_999, 2),
        (2_160_000, 3),
        (3_599_999, 3),
        (3_600_000, 4),
    ] {
        assert_eq!(state.view("2026-11-14", ms).unwrap().stage, stage);
    }
}
#[test]
fn achievement_is_once_per_day_and_disabling_target_never_unlocks_choice() {
    let state = Document::default()
        .apply(
            &Command::Choose {
                plant: Plant::Sprout,
            },
            0,
            context("2026-11-14"),
        )
        .unwrap();
    let achieved = state.observe(context("2026-11-14"), 3_600_000).unwrap();
    assert_eq!(achieved.revision, 2);
    assert_eq!(
        achieved.observe(context("2026-11-14"), 3_700_000).unwrap(),
        achieved
    );
    let reduced = achieved.observe(context("2026-11-14"), 0).unwrap();
    assert_eq!(reduced, achieved);
    let disabled = achieved
        .apply(
            &Command::SetGoal { minutes: None },
            2,
            context("2026-11-14"),
        )
        .unwrap();
    assert!(!disabled.view("2026-11-14", 3_600_000).unwrap().reached);
    assert_eq!(
        disabled.observe(context("2026-11-14"), 4_000_000).unwrap(),
        disabled
    );
    assert!(disabled
        .apply(
            &Command::Choose { plant: Plant::Corn },
            3,
            context("2026-11-14")
        )
        .is_err());
    assert_eq!(disabled.days["2026-11-14"].goal_on_celebration, Some(60));
}
