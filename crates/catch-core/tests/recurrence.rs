use catch_core::{
    recurrence::{Candidate, Command, Document, Schedule},
    todos::{self, calendar_date, Fields},
};
fn fields(text: &str) -> Fields {
    Fields {
        text: text.into(),
        notes: "备注".into(),
        urgent: false,
        deadline: None,
    }
}
fn create(schedule: Schedule, start: &str, today: &str) -> Candidate {
    Document::default()
        .command(
            &Command::Create {
                fields: fields("阅读"),
                resources: vec![],
                schedule,
                starts_on: start.into(),
            },
            &todos::Document::default(),
            0,
            today,
            "rule-1",
        )
        .unwrap()
}
fn complete(candidate: Candidate, today: &str) -> Candidate {
    let id = candidate.todos.todos[0].id.clone();
    let after = candidate
        .todos
        .apply(
            &todos::Command::Complete { id, done: true },
            candidate.todos.revision,
            todos::Context {
                now: 0,
                local_day: today,
                new_id: "unused",
            },
        )
        .unwrap();
    let rules = candidate
        .rules
        .after_todo_change(&candidate.todos, &after, today)
        .unwrap();
    Candidate {
        rules,
        todos: after,
    }
}
#[test]
fn long_absence_generates_one_latest_occurrence_and_pending_work_never_multiplies() {
    let candidate = create(Schedule::Daily, "2026-01-01", "2026-11-14");
    assert_eq!(candidate.todos.todos.len(), 1);
    assert_eq!(
        candidate.todos.todos[0]
            .recurrence
            .as_ref()
            .unwrap()
            .scheduled_on,
        "2026-11-14"
    );
    let later = candidate
        .rules
        .reconcile(&candidate.todos, "2027-02-01")
        .unwrap();
    assert_eq!(later.todos, candidate.todos);
    let finished = complete(later, "2027-02-01");
    let same_day = finished
        .rules
        .reconcile(&finished.todos, "2027-02-01")
        .unwrap();
    assert_eq!(same_day.todos.todos.len(), 1);
    let tomorrow = same_day
        .rules
        .reconcile(&same_day.todos, "2027-02-02")
        .unwrap();
    assert_eq!(tomorrow.todos.todos.len(), 2);
    assert_ne!(tomorrow.todos.todos[0].id, tomorrow.todos.todos[1].id);
    assert!(tomorrow.todos.todos[1].done);
}
#[test]
fn monthly_day_31_clamps_in_february_and_returns_to_march_31() {
    let candidate = create(Schedule::Monthly { day: 31 }, "2026-01-31", "2026-01-31");
    assert_eq!(candidate.rules.templates[0].next_on, "2026-02-28");
    let done = complete(candidate, "2026-01-31");
    let feb = done.rules.reconcile(&done.todos, "2026-02-28").unwrap();
    assert_eq!(feb.todos.todos[0].deadline.as_deref(), Some("2026-02-28"));
    assert_eq!(feb.rules.templates[0].next_on, "2026-03-31");
    assert_eq!(
        Schedule::Monthly { day: 31 }
            .on_or_after(
                calendar_date("2028-01-01").unwrap(),
                calendar_date("2028-02-01").unwrap()
            )
            .unwrap(),
        calendar_date("2028-02-29").unwrap()
    );
}
#[test]
fn weekly_and_every_n_days_follow_the_original_anchor_without_drift() {
    let anchor = calendar_date("2026-11-02").unwrap();
    assert_eq!(
        Schedule::Weekly {
            weekdays: vec![0, 4]
        }
        .on_or_after(anchor, calendar_date("2026-11-03").unwrap())
        .unwrap(),
        calendar_date("2026-11-06").unwrap()
    );
    assert_eq!(
        Schedule::Every { days: 3 }
            .latest(anchor, calendar_date("2026-11-14").unwrap())
            .unwrap(),
        Some(calendar_date("2026-11-14").unwrap())
    );
    assert_eq!(
        Schedule::Every { days: 3 }
            .on_or_after(anchor, calendar_date("2026-11-15").unwrap())
            .unwrap(),
        calendar_date("2026-11-17").unwrap()
    );
    assert!(Schedule::Every { days: 0 }.validate().is_err());
    assert!(Schedule::Weekly {
        weekdays: vec![1, 1]
    }
    .validate()
    .is_err());
}
#[test]
fn undo_cannot_create_two_unfinished_occurrences_and_clock_rollback_cannot_repeat_a_date() {
    let candidate = complete(
        create(Schedule::Daily, "2026-11-14", "2026-11-14"),
        "2026-11-14",
    );
    let tomorrow = candidate
        .rules
        .reconcile(&candidate.todos, "2026-11-15")
        .unwrap();
    let old = tomorrow.todos.todos[1].id.clone();
    let undone = tomorrow
        .todos
        .apply(
            &todos::Command::Complete {
                id: old,
                done: false,
            },
            tomorrow.todos.revision,
            todos::Context {
                now: 0,
                local_day: "2026-11-15",
                new_id: "unused",
            },
        )
        .unwrap();
    assert!(tomorrow
        .rules
        .after_todo_change(&tomorrow.todos, &undone, "2026-11-15")
        .is_err());
    assert_eq!(
        tomorrow
            .rules
            .reconcile(&tomorrow.todos, "2026-11-14")
            .unwrap()
            .todos,
        tomorrow.todos
    );
}
#[test]
fn stop_and_this_and_future_edits_keep_completed_history_independent() {
    let candidate = complete(
        create(Schedule::Daily, "2026-11-14", "2026-11-14"),
        "2026-11-14",
    );
    let tomorrow = candidate
        .rules
        .reconcile(&candidate.todos, "2026-11-15")
        .unwrap();
    let updated = tomorrow
        .rules
        .command(
            &Command::Update {
                id: "rule-1".into(),
                fields: fields("改为写作"),
                resources: vec![],
                schedule: Schedule::Weekly { weekdays: vec![0] },
            },
            &tomorrow.todos,
            tomorrow.rules.revision,
            "2026-11-15",
            "unused",
        )
        .unwrap();
    assert_eq!(updated.todos.todos[0].text, "改为写作");
    assert_eq!(updated.todos.todos[1].text, "阅读");
    let stopped = updated
        .rules
        .command(
            &Command::Stop {
                id: "rule-1".into(),
            },
            &updated.todos,
            updated.rules.revision,
            "2026-11-15",
            "unused",
        )
        .unwrap();
    let done = complete(stopped, "2026-11-15");
    assert_eq!(
        done.rules
            .reconcile(&done.todos, "2027-01-01")
            .unwrap()
            .todos,
        done.todos
    );
}

#[test]
fn converting_a_todo_preserves_its_identity_and_does_not_emit_a_duplicate() {
    let todos = todos::Document::default()
        .apply(
            &todos::Command::Create(fields("原来的待办")),
            0,
            todos::Context {
                now: 0,
                local_day: "2026-11-14",
                new_id: "existing",
            },
        )
        .unwrap();
    let candidate = Document::default()
        .command(
            &Command::Convert {
                id: "existing".into(),
                schedule: Schedule::Daily,
                starts_on: "2026-11-14".into(),
            },
            &todos,
            0,
            "2026-11-14",
            "rule-1",
        )
        .unwrap();
    assert_eq!(candidate.todos.todos.len(), 1);
    assert_eq!(candidate.todos.todos[0].id, "existing");
    assert_eq!(candidate.todos.todos[0].notes, "备注");
    let done = complete(candidate, "2026-11-14");
    let tomorrow = done.rules.reconcile(&done.todos, "2026-11-15").unwrap();
    assert_eq!(tomorrow.todos.todos.len(), 2);
    assert_eq!(tomorrow.todos.todos[1].id, "existing");
    assert_eq!(tomorrow.todos.todos[0].text, "原来的待办");
}
