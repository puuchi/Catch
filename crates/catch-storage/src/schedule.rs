//! A recoverable two-file commit keeps recurrence cursors and emitted todos in
//! one logical transaction while retaining the existing todos.json array.
use crate::{encode, Disk, Result};
use catch_core::{recurrence, todos};
use serde::{Deserialize, Serialize};
pub(super) const RULES: &str = "catch-recurrences.json";
pub(super) const PENDING: &str = "catch-schedule-pending.json";
#[derive(Serialize, Deserialize)]
struct Change {
    file: String,
    before: Option<String>,
    after: String,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    version: u32,
    changes: Vec<Change>,
}
pub(super) fn read(disk: &dyn Disk) -> Result<recurrence::Document> {
    let state = if let Some(bytes) = disk.read(RULES)? {
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("周期数据无法读取，原件已保留：{error}"))?
    } else {
        if disk.read("catch-recurrences.json.bak")?.is_some() {
            return Err("周期主文件缺失但备份存在，请先恢复文件".into());
        }
        recurrence::Document::default()
    };
    state.validate()?;
    Ok(state)
}
fn strings(bytes: Option<Vec<u8>>) -> Result<Option<String>> {
    bytes
        .map(|bytes| String::from_utf8(bytes).map_err(|_| "周期事务原件不是 UTF-8".into()))
        .transpose()
}
fn validate_pair(journal: &Journal, after: bool) -> Result<()> {
    let get = |key: &str| -> Option<&str> {
        journal
            .changes
            .iter()
            .find(|change| change.file == key)
            .and_then(|change| {
                if after {
                    Some(change.after.as_str())
                } else {
                    change.before.as_deref()
                }
            })
    };
    let todos = if let Some(raw) = get("todos.json") {
        serde_json::from_str(raw).map_err(|error| format!("周期事务待办无效：{error}"))?
    } else {
        vec![]
    };
    let rules: recurrence::Document = if let Some(raw) = get(RULES) {
        serde_json::from_str(raw).map_err(|error| format!("周期事务规则无效：{error}"))?
    } else {
        recurrence::Document::default()
    };
    rules.validate_links(&todos::Document {
        todos,
        archive: vec![],
        revision: 0,
    })
}
pub(super) fn recover(disk: &dyn Disk) -> Result<()> {
    let Some(bytes) = disk.read(PENDING)? else {
        return Ok(());
    };
    if disk.read("catch-archive-pending.json")?.is_some() {
        return Err("存在两份未完成事务，已停止恢复，请保留文件检查".into());
    }
    let journal: Journal =
        serde_json::from_slice(&bytes).map_err(|error| format!("周期恢复日志无效：{error}"))?;
    if journal.version != 1
        || journal.changes.len() != 2
        || journal
            .changes
            .iter()
            .filter(|change| change.file == RULES)
            .count()
            != 1
        || journal
            .changes
            .iter()
            .filter(|change| change.file == "todos.json")
            .count()
            != 1
    {
        return Err("周期恢复日志版本或文件集合无效".into());
    }
    validate_pair(&journal, false)?;
    validate_pair(&journal, true)?;
    // Check all preconditions before writing either side.
    for change in &journal.changes {
        let current = strings(disk.read(&change.file)?)?;
        if current != change.before && current.as_deref() != Some(change.after.as_str()) {
            return Err("周期事务期间文件被外部修改，已停止恢复".into());
        }
    }
    for change in &journal.changes {
        if strings(disk.read(&change.file)?)?.as_deref() == Some(change.after.as_str()) {
            continue;
        }
        if let Some(before) = &change.before {
            disk.write_atomic(&format!("{}.bak", change.file), before.as_bytes())?;
        }
        disk.write_atomic(&change.file, change.after.as_bytes())?;
    }
    disk.remove(PENDING)
}
pub(super) fn write(disk: &dyn Disk, candidate: &recurrence::Candidate) -> Result<()> {
    candidate.rules.validate_links(&candidate.todos)?;
    let journal = Journal {
        version: 1,
        changes: vec![
            Change {
                file: RULES.into(),
                before: strings(disk.read(RULES)?)?,
                after: String::from_utf8(encode(&candidate.rules)?)
                    .map_err(|error| error.to_string())?,
            },
            Change {
                file: "todos.json".into(),
                before: strings(disk.read("todos.json")?)?,
                after: String::from_utf8(encode(&candidate.todos.todos)?)
                    .map_err(|error| error.to_string())?,
            },
        ],
    };
    validate_pair(&journal, false)?;
    validate_pair(&journal, true)?;
    disk.write_atomic(PENDING, &encode(&journal)?)?;
    recover(disk)
}
