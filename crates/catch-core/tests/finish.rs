use catch_core::tracking::{Command, Context, Document, Status};
fn clock(now: i64) -> Context<'static> {
    Context {
        now,
        new_id: "matter",
    }
}
fn started() -> Document {
    Document::default()
        .apply(
            &Command::Start {
                name: "work".into(),
                todo_id: None,
                resources: vec![],
            },
            0,
            clock(0),
        )
        .unwrap()
}
fn session(doc: &Document) -> &str {
    &doc.activities[0]
        .sessions
        .as_ref()
        .unwrap()
        .last()
        .unwrap()
        .id
}
#[test]
fn empty_next_session_still_rejects_old_identity_and_revision() {
    for progress in ["", "   ", "record"] {
        let before = started();
        let command = Command::NextSession {
            activity_id: "matter".into(),
            session_id: session(&before).into(),
            progress: progress.into(),
        };
        let next = before
            .apply(&command, before.revision, clock(1000))
            .unwrap();
        let sessions = next.activities[0].sessions.as_ref().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].progress, progress.trim());
        assert!(next.apply(&command, before.revision, clock(2000)).is_err());
        assert!(next.apply(&command, next.revision, clock(2000)).is_err());
        assert_eq!(next.activities[0].sessions.as_ref().unwrap().len(), 2);
    }
    let before = started();
    let command: Command = serde_json::from_value(serde_json::json!({
        "type": "next-session", "payload": {"activityId": "matter", "sessionId": session(&before)}
    }))
    .unwrap();
    assert!(before.apply(&command, before.revision, clock(1000)).is_ok());
}
#[test]
fn finish_saves_only_the_last_owned_session_in_one_revision() {
    let before = started();
    let next = before
        .apply(
            &Command::NextSession {
                activity_id: "matter".into(),
                session_id: session(&before).into(),
                progress: "first".into(),
            },
            before.revision,
            clock(1000),
        )
        .unwrap();
    for id in [session(&before), "foreign:0"] {
        assert!(next
            .finish(
                "matter",
                Some(id),
                Some("draft"),
                next.revision,
                clock(2000)
            )
            .is_err());
    }
    assert!(next
        .finish("matter", None, Some("draft"), next.revision, clock(2000))
        .is_err());
    assert!(next
        .finish(
            "matter",
            Some(session(&next)),
            Some(&"x".repeat(2001)),
            next.revision,
            clock(2000)
        )
        .is_err());
    assert!(next.finish("matter", None, None, 0, clock(2000)).is_err());
    let result = next
        .finish(
            "matter",
            Some(session(&next)),
            Some(" final "),
            next.revision,
            clock(2000),
        )
        .unwrap();
    assert_eq!(result.revision, next.revision + 1);
    assert_eq!(result.activities[0].status, Status::Completed);
    let rows = result.activities[0].sessions.as_ref().unwrap();
    assert_eq!(rows[0].progress, "first");
    assert_eq!(rows[1].progress, "final");
    assert!(rows[1].end_time.is_some());
    assert_eq!(next.activities[0].status, Status::Running);
}
#[test]
fn paused_finish_can_clear_progress_and_legacy_end_remains_supported() {
    let running = started();
    let paused = running
        .apply(
            &Command::Pause {
                reason: "manual".into(),
                cutoff: None,
            },
            running.revision,
            clock(1000),
        )
        .unwrap();
    let finished = paused
        .finish(
            "matter",
            Some(session(&paused)),
            Some(""),
            paused.revision,
            clock(2000),
        )
        .unwrap();
    assert_eq!(finished.activities[0].status, Status::Completed);
    assert_eq!(
        finished.activities[0].sessions.as_ref().unwrap()[0].progress,
        ""
    );
    assert!(finished
        .finish("matter", None, None, finished.revision, clock(3000))
        .is_err());
    let legacy = paused
        .apply(
            &Command::End {
                id: Some("matter".into()),
            },
            paused.revision,
            clock(2000),
        )
        .unwrap();
    assert_eq!(legacy.activities[0].status, Status::Completed);
}
