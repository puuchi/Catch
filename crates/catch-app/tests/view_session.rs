use catch_app::view_session::{Phase, Session, UiState};
use catch_storage::{Disk, FileDisk};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
struct Failing {
    inner: FileDisk,
    fail: AtomicBool,
}
impl Disk for Failing {
    fn read(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        self.inner.read(key)
    }
    fn remove(&self, key: &str) -> Result<(), String> {
        self.inner.remove(key)
    }
    fn write_atomic(&self, key: &str, value: &[u8]) -> Result<(), String> {
        if self.fail.load(Ordering::SeqCst) {
            Err("disk full".into())
        } else {
            self.inner.write_atomic(key, value)
        }
    }
}
#[test]
fn collapse_requires_saved_drafts_and_reopening_restores_page_and_unsubmitted_text() {
    let dir = tempfile::tempdir().unwrap();
    let disk = Arc::new(FileDisk::new(dir.path()).unwrap());
    let mut session = Session::open(disk.clone()).unwrap();
    let first = session.begin_open().unwrap().unwrap();
    assert!(session.painted(first));
    let state:UiState=serde_json::from_value(serde_json::json!({"version":1,"page":"current","activityName":"还没提交","editor":{"progress":"编辑到一半"},"scroll":{"current":120}})).unwrap();
    session.prepare_collapse(first, state).unwrap();
    assert_eq!(session.phase(), Phase::Collapsing);
    assert!(session.finish_collapse(first));
    let second = session.begin_open().unwrap().unwrap();
    assert!(second > first);
    assert_eq!(session.saved().body["activityName"], "还没提交");
    assert!(!session.painted(first));
    assert!(!session.failed_open(first));
    assert!(session.painted(second));
    drop(session);
    let restored = Session::open(disk).unwrap();
    assert_eq!(restored.saved().body["editor"]["progress"], "编辑到一半");
}
#[test]
fn failed_draft_save_retains_the_expanded_window_and_prior_disk_state() {
    let dir = tempfile::tempdir().unwrap();
    let disk = Arc::new(Failing {
        inner: FileDisk::new(dir.path()).unwrap(),
        fail: AtomicBool::new(false),
    });
    let mut session = Session::open(disk.clone()).unwrap();
    let generation = session.begin_open().unwrap().unwrap();
    session.painted(generation);
    session.stash(generation, UiState::default()).unwrap();
    let before = disk.read("catch-ui-state.json").unwrap();
    disk.fail.store(true, Ordering::SeqCst);
    let mut state = UiState::default();
    state.body.insert("activityName".into(), "新草稿".into());
    assert!(session.prepare_collapse(generation, state).is_err());
    assert_eq!(session.phase(), Phase::Expanded);
    assert_eq!(disk.read("catch-ui-state.json").unwrap(), before);
}
#[test]
fn timed_out_windows_cannot_hide_or_overwrite_a_new_generation() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(Arc::new(FileDisk::new(dir.path()).unwrap())).unwrap();
    let stale = session.begin_open().unwrap().unwrap();
    assert!(session.failed_open(stale));
    let current = session.begin_open().unwrap().unwrap();
    assert!(!session.painted(stale));
    assert!(session.stash(stale, UiState::default()).is_err());
    assert!(session.painted(current));
    assert!(!session.failed_open(stale));
    assert!(!session.finish_collapse(stale));
    assert_eq!(session.phase(), Phase::Expanded);
}
