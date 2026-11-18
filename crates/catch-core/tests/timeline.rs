use catch_core::tracking::{
    iso, timestamp, Command, Context, Document, Matter, Segment, Session, Status,
};
use serde_json::{json, Value};

fn t(n: i64) -> String {
    iso(timestamp("2026-11-14T23:00:00Z").unwrap() + n * 60_000).unwrap()
}
fn clock(n: i64) -> Context<'static> {
    Context {
        now: timestamp(&t(n)).unwrap(),
        new_id: "unique-token",
    }
}
fn row(
    id: &str,
    start: i64,
    end: Option<i64>,
    intervals: &[(i64, Option<i64>)],
    progress: &str,
) -> Session {
    Session {
        id: id.into(),
        start_time: t(start),
        end_time: end.map(t),
        progress: progress.into(),
        segments: intervals
            .iter()
            .map(|(a, b)| Segment::new(&t(*a), b.map(t).as_deref()))
            .collect(),
        extra: Default::default(),
    }
}
fn matter(id: &str, rows: Vec<Session>) -> Matter {
    let running = rows.iter().any(|r| r.end_time.is_none());
    let mut flat: Vec<_> = rows.iter().flat_map(|r| r.segments.clone()).collect();
    flat.sort_by_key(|s| timestamp(&s.start).unwrap());
    serde_json::from_value(json!({ "id":id, "name":id, "startTime":rows[0].start_time,
        "endTime":if running { None } else { rows.last().unwrap().end_time.clone() },
        "status":if running { "running" } else { "completed" }, "segments":flat,
        "sessions":rows, "nextSessionIndex":10, "todoId":null,
        "createdAt":t(0), "updatedAt":t(0), "pauseReason":null }))
    .unwrap()
}
fn document(matters: Vec<Matter>) -> Document {
    let doc = Document {
        activities: matters,
        ..Default::default()
    };
    doc.validate().unwrap();
    doc
}
fn command(kind: &str, payload: Value) -> Command {
    serde_json::from_value(json!({"type":kind,"payload":payload})).unwrap()
}
fn apply(doc: &Document, kind: &str, payload: Value, now: i64) -> Document {
    let result = doc
        .apply(&command(kind, payload), doc.revision, clock(now))
        .unwrap();
    assert_eq!(result.revision, doc.revision + 1);
    result.validate().unwrap();
    result
}
fn merge(target: &str) -> Value {
    json!({"first":{"activityId":"a","sessionId":"a:1"}, "second":{"activityId":"b","sessionId":"b:0"}, "targetActivityId":target})
}
fn fixture(running: bool) -> Document {
    document(vec![
        matter(
            "a",
            vec![
                row("a:0", 0, Some(10), &[(0, Some(10))], "unrelated a"),
                row(
                    "a:1",
                    50,
                    Some(70),
                    &[(50, Some(55)), (60, Some(70))],
                    "first",
                ),
            ],
        ),
        matter(
            "b",
            vec![row(
                "b:0",
                80,
                if running { None } else { Some(100) },
                &[(80, if running { None } else { Some(100) })],
                "second",
            )],
        ),
    ])
}
fn total(doc: &Document, now: i64) -> i64 {
    doc.activities
        .iter()
        .map(|m| m.duration(clock(now).now, None).unwrap())
        .sum()
}
#[test]
fn merge_cross_day_only_selected_sessions_and_preserves_effective_pause_intervals() {
    let mut doc = fixture(false);
    doc.activities[1].sessions.as_mut().unwrap().push(row(
        "b:1",
        110,
        Some(120),
        &[(110, Some(120))],
        "unrelated b",
    ));
    doc.activities[1]
        .segments
        .push(Segment::new(&t(110), Some(&t(120))));
    doc.activities[1].end_time = Some(t(120));
    let before_a = doc.activities[0].sessions.as_ref().unwrap()[0].clone();
    let before_b = doc.activities[1].sessions.as_ref().unwrap()[1].clone();
    let merged = apply(&doc, "merge-sessions", merge("a"), 130);
    assert_eq!(total(&merged, 130), total(&doc, 130));
    assert_eq!(merged.activities[0].sessions.as_ref().unwrap()[0], before_a);
    assert_eq!(merged.activities[1].sessions.as_ref().unwrap()[0], before_b);
    let row = &merged.activities[0].sessions.as_ref().unwrap()[1];
    assert_eq!(row.id, "a:1");
    assert_eq!(row.progress, "first\nsecond");
    assert_eq!(row.segments.len(), 3);
    assert_eq!(row.start_time, t(50));
    assert_eq!(row.end_time, Some(t(100)));
    assert_eq!(merged.activities[0].next_session_index, Some(10));
    assert!(merged
        .apply(
            &command("merge-sessions", merge("a")),
            merged.revision,
            clock(130)
        )
        .is_err());
}
#[test]
fn resources_union_deduplicates_kind_target_and_repairs_id_collisions() {
    use catch_core::resources::{Kind, Resource};
    let mut doc = fixture(false);
    doc.activities[0].resources =
        vec![Resource::new("same", Kind::File, r"C:\Work\One", None).unwrap()];
    doc.activities[1].resources = vec![
        Resource::new("dup", Kind::File, "c:/work/ONE", None).unwrap(),
        Resource::new("same", Kind::Web, "https://example.com", None).unwrap(),
    ];
    let merged = apply(&doc, "merge-sessions", merge("a"), 130);
    assert_eq!(merged.activities.len(), 1);
    let resources = &merged.activities[0].resources;
    assert_eq!(resources.len(), 2);
    assert_ne!(resources[0].id, resources[1].id);
    assert_eq!(resources[0], doc.activities[0].resources[0]);
}
#[test]
fn merge_todo_policy_never_loses_an_association_and_target_must_be_selected() {
    for (target, source, ok) in [
        (None, None, true),
        (Some("todo"), None, true),
        (None, Some("todo"), false),
        (Some("one"), Some("two"), false),
        (Some("same"), Some("same"), true),
    ] {
        let mut doc = fixture(false);
        doc.activities[0].todo_id = target.map(str::to_owned);
        doc.activities[1].todo_id = source.map(str::to_owned);
        assert_eq!(
            doc.apply(&command("merge-sessions", merge("a")), 0, clock(130))
                .is_ok(),
            ok
        );
    }
    assert!(fixture(false)
        .apply(&command("merge-sessions", merge("outsider")), 0, clock(130))
        .is_err());
}
#[test]
fn global_adjacency_rejects_intervening_sessions_even_inside_pause() {
    for (start, end) in [(72, 75), (56, 59)] {
        let mut doc = fixture(false);
        doc.activities.push(matter(
            "c",
            vec![row("c:0", start, Some(end), &[(start, Some(end))], "")],
        ));
        doc.validate().unwrap();
        assert!(doc
            .apply(&command("merge-sessions", merge("a")), 0, clock(130))
            .is_err());
    }
}
#[test]
fn merge_running_transfers_only_the_selected_open_session_and_has_one_owner() {
    for target in ["a", "b"] {
        let doc = fixture(true);
        let merged = apply(&doc, "merge-sessions", merge(target), 130);
        assert_eq!(merged.current().unwrap().id, target);
        assert_eq!(total(&merged, 140), total(&doc, 140));
        let running = merged
            .current()
            .unwrap()
            .sessions
            .as_ref()
            .unwrap()
            .iter()
            .find(|r| r.end_time.is_none())
            .unwrap();
        assert_eq!(running.id, if target == "a" { "a:1" } else { "b:0" });
        assert!(running.segments.last().unwrap().end.is_none());
    }
}
#[test]
fn explicit_gap_adds_only_requested_uncovered_adjacent_time() {
    let doc = fixture(false);
    let payload = json!({"first":{"activityId":"a","sessionId":"a:1"},"gap":{"startTime":t(70),"endTime":t(80)},"targetActivityId":"a"});
    let merged = apply(&doc, "merge-sessions", payload, 130);
    assert_eq!(total(&merged, 130), total(&doc, 130) + 10 * 60_000);
    for (start, end) in [(69, 80), (70, 81), (71, 80), (49, 51), (80, 70)] {
        let payload = json!({"first":{"activityId":"a","sessionId":"a:1"},"gap":{"startTime":t(start),"endTime":t(end)},"targetActivityId":"a"});
        assert!(doc
            .apply(&command("merge-sessions", payload), 0, clock(130))
            .is_err());
    }
    let running = fixture(true);
    let payload = json!({"first":{"activityId":"b","sessionId":"b:0"},"gap":{"startTime":t(70),"endTime":t(80)},"targetActivityId":"b"});
    let merged = apply(&running, "merge-sessions", payload, 130);
    assert_eq!(merged.current().unwrap().id, "b");
    assert_eq!(total(&merged, 140), total(&running, 140) + 10 * 60_000);
}
#[test]
fn split_across_midnight_pause_and_running_conserves_duration_and_monotonic_ids() {
    for running in [false, true] {
        for at in [54, 58, 65] {
            let end = if running { None } else { Some(80) };
            let doc = document(vec![matter(
                "a",
                vec![row(
                    "a:0",
                    50,
                    end,
                    &[(50, Some(55)), (60, end)],
                    "original",
                )],
            )]);
            let split = apply(
                &doc,
                "split-session",
                json!({"activityId":"a","sessionId":"a:0","atTime":t(at)}),
                100,
            );
            let rows = split.activities[0].sessions.as_ref().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].id, "a:0");
            assert_eq!(rows[1].id, "a:10");
            assert_eq!(rows[0].progress, "original");
            assert_eq!(rows[1].progress, "");
            assert_eq!(rows[1].end_time, end.map(t));
            assert_eq!(split.activities[0].next_session_index, Some(11));
            assert_eq!(total(&doc, 110), total(&split, 110));
        }
    }
}
#[test]
fn split_rejects_boundaries_future_and_pause_only_halves_without_mutating_input() {
    let doc = document(vec![matter(
        "a",
        vec![row("a:0", 50, Some(80), &[(60, Some(70))], "")],
    )]);
    let original = doc.clone();
    for at in [50, 55, 60, 70, 75, 80, 100] {
        assert!(doc
            .apply(
                &command(
                    "split-session",
                    json!({"activityId":"a","sessionId":"a:0","atTime":t(at)})
                ),
                0,
                clock(90)
            )
            .is_err());
        assert_eq!(doc, original);
    }
    assert!(doc
        .apply(
            &command(
                "split-session",
                json!({"activityId":"a","sessionId":"a:0","atTime":t(65)})
            ),
            1,
            clock(90)
        )
        .is_err());
}
fn started() -> Document {
    document(vec![matter(
        "a",
        vec![row("a:0", 0, None, &[(0, None)], "old")],
    )])
}
fn begin(doc: &Document) -> Document {
    apply(
        doc,
        "begin-session-edit",
        json!({"activityId":"a","sessionId":"a:0","progress":"draft"}),
        10,
    )
}
fn finish_payload(cancel: bool) -> Value {
    json!({"activityId":"a","sessionId":"a:0","editToken":"edit:unique-token","cancel":cancel,"progress":"changed"})
}
#[test]
fn begin_edit_pauses_and_cancel_or_save_resumes_the_same_session_not_editing_time() {
    for cancel in [true, false] {
        let doc = started();
        let paused = begin(&doc);
        assert_eq!(paused.activities[0].status, Status::Paused);
        assert_eq!(
            paused.activities[0].pause_reason.as_deref(),
            Some("edit:unique-token")
        );
        let finished = apply(&paused, "finish-session-edit", finish_payload(cancel), 20);
        let matter = &finished.activities[0];
        let rows = matter.sessions.as_ref().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "a:0");
        assert_eq!(rows[0].progress, if cancel { "draft" } else { "changed" });
        assert_eq!(rows[0].segments.len(), 2);
        assert_eq!(rows[0].segments[0].end, Some(t(10)));
        assert_eq!(rows[0].segments[1].start, t(20));
        assert_eq!(matter.next_session_index, Some(10));
        assert_eq!(matter.duration(clock(30).now, None).unwrap(), 20 * 60_000);
        assert!(matter.pause_reason.is_none());
        assert!(!matter.extra.contains_key("catchSessionEditSnapshot"));
        assert!(finished
            .apply(
                &command("finish-session-edit", finish_payload(cancel)),
                finished.revision,
                clock(30)
            )
            .is_err());
    }
}
#[test]
fn save_uses_legacy_segment_validation_and_can_retry_after_failure() {
    let paused = begin(&started());
    let original = paused.clone();
    for changes in [
        json!({"endTime":t(30)}),
        json!({"progress":"x".repeat(2001)}),
        json!({"segments":[{"start":t(0),"end":t(15)}]}),
        json!({"startTime":t(11)}),
    ] {
        let mut payload = finish_payload(false);
        payload
            .as_object_mut()
            .unwrap()
            .extend(changes.as_object().unwrap().clone());
        assert!(paused
            .apply(
                &command("finish-session-edit", payload),
                paused.revision,
                clock(20)
            )
            .is_err());
        assert_eq!(paused, original);
    }
    let mut payload = finish_payload(false);
    payload["startTime"] = json!(t(2));
    payload["endTime"] = json!(t(9));
    payload["segments"] = json!([{"start":t(2),"end":t(9)}]);
    let saved = apply(&paused, "finish-session-edit", payload, 20);
    assert_eq!(total(&saved, 30), 17 * 60_000);
    assert!(paused
        .apply(
            &command("finish-session-edit", finish_payload(true)),
            paused.revision,
            clock(5)
        )
        .is_err());
}
#[test]
fn token_and_session_modifications_are_detected_without_overwriting_anyones_work() {
    let paused = begin(&started());
    let mut bad = finish_payload(true);
    bad["editToken"] = json!("edit:wrong");
    assert!(paused
        .apply(
            &command("finish-session-edit", bad),
            paused.revision,
            clock(20)
        )
        .is_err());
    let modified = apply(
        &paused,
        "set-progress",
        json!({"activityId":"a","sessionId":"a:0","progress":"other window"}),
        15,
    );
    assert!(modified
        .apply(
            &command("finish-session-edit", finish_payload(true)),
            modified.revision,
            clock(20)
        )
        .is_err());
    assert_eq!(
        modified.activities[0].sessions.as_ref().unwrap()[0].progress,
        "other window"
    );
    assert!(paused
        .apply(
            &command("finish-session-edit", finish_payload(true)),
            0,
            clock(20)
        )
        .is_err());
}
#[test]
fn another_running_matter_is_never_stolen_on_cancel_or_save() {
    for cancel in [true, false] {
        let paused = begin(&started());
        let other = paused
            .apply(
                &Command::Start {
                    name: "other".into(),
                    todo_id: None,
                    resources: vec![],
                },
                paused.revision,
                Context {
                    now: clock(15).now,
                    new_id: "b",
                },
            )
            .unwrap();
        let finished = apply(&other, "finish-session-edit", finish_payload(cancel), 20);
        assert_eq!(finished.current().unwrap().id, "b");
        assert_eq!(finished.activities[0].status, Status::Paused);
        assert_eq!(
            finished.activities[0].pause_reason.as_deref(),
            Some("manual")
        );
        assert_eq!(finished.activities[0].sessions.as_ref().unwrap().len(), 1);
        assert_eq!(total(&finished, 30), 25 * 60_000);
    }
}
#[test]
fn begin_failure_does_not_pause_and_only_running_last_session_is_eligible() {
    let doc = started();
    assert!(doc
        .apply(
            &command(
                "begin-session-edit",
                json!({"activityId":"a","sessionId":"a:0","progress":"x".repeat(2001)})
            ),
            0,
            clock(10)
        )
        .is_err());
    assert_eq!(doc.current().unwrap().id, "a");
    assert!(doc
        .apply(
            &command(
                "begin-session-edit",
                json!({"activityId":"a","sessionId":"wrong"})
            ),
            0,
            clock(10)
        )
        .is_err());
    let paused = begin(&doc);
    assert!(paused
        .apply(
            &command(
                "begin-session-edit",
                json!({"activityId":"a","sessionId":"a:0"})
            ),
            paused.revision,
            clock(20)
        )
        .is_err());
}

#[test]
fn same_matter_merge_is_order_independent_and_split_does_not_reuse_deleted_identity() {
    let doc = document(vec![matter(
        "a",
        vec![
            row("a:0", 0, Some(10), &[(0, Some(10))], "first"),
            row("a:1", 20, Some(30), &[(20, Some(30))], "second"),
        ],
    )]);
    let payload = json!({"first":{"activityId":"a","sessionId":"a:1"},"second":{"activityId":"a","sessionId":"a:0"},"targetActivityId":"a"});
    let merged = apply(&doc, "merge-sessions", payload, 40);
    let rows = merged.activities[0].sessions.as_ref().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "a:1");
    assert_eq!(rows[0].progress, "first\nsecond");
    let split = apply(
        &merged,
        "split-session",
        json!({"activityId":"a","sessionId":"a:1","atTime":t(15)}),
        40,
    );
    assert_eq!(split.activities[0].sessions.as_ref().unwrap()[1].id, "a:10");
    assert_eq!(total(&split, 40), total(&doc, 40));
}

#[test]
fn unrelated_legacy_history_and_extensions_are_unchanged_by_adjacency_checks() {
    let mut doc = fixture(false);
    let mut legacy = matter(
        "legacy",
        vec![row("legacy:0", 140, Some(150), &[(140, Some(150))], "")],
    );
    legacy.sessions = None;
    legacy.next_session_index = None;
    legacy
        .extra
        .insert("legacyExtra".into(), json!({"keep":true}));
    doc.activities.push(legacy.clone());
    let merged = apply(&doc, "merge-sessions", merge("a"), 160);
    assert_eq!(merged.activities.last().unwrap(), &legacy);
    for payload in [
        json!({"first":{"activityId":"a","sessionId":"b:0"},"second":{"activityId":"b","sessionId":"b:0"},"targetActivityId":"a"}),
        json!({"first":{"activityId":"a","sessionId":"a:1"},"targetActivityId":"a"}),
        json!({"first":{"activityId":"a","sessionId":"a:1"},"second":{"activityId":"b","sessionId":"b:0"},"gap":{"startTime":t(70),"endTime":t(80)},"targetActivityId":"a"}),
    ] {
        assert!(doc
            .apply(
                &command("merge-sessions", payload),
                doc.revision,
                clock(160)
            )
            .is_err());
    }
}

#[test]
fn repeated_edit_cycles_preserve_pause_segments_and_same_session_count() {
    let paused = begin(&started());
    let resumed = apply(&paused, "finish-session-edit", finish_payload(true), 20);
    let paused_again = apply(
        &resumed,
        "begin-session-edit",
        json!({"activityId":"a","sessionId":"a:0"}),
        30,
    );
    let resumed_again = apply(
        &paused_again,
        "finish-session-edit",
        finish_payload(false),
        40,
    );
    let rows = resumed_again.activities[0].sessions.as_ref().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].segments.len(), 3);
    assert_eq!(total(&resumed_again, 50), 30 * 60_000);
    assert_eq!(resumed_again.activities[0].next_session_index, Some(10));
}
