//! Read-only legacy snapshots are copied into an empty independent profile.
//! No source path is opened for writing; the host owns the preview/commit check.
use crate::{encode, tracking::decode_import, Disk, Result};
use catch_core::{todos, tracking};
use serde::{Deserialize, Serialize};
pub const FILES: [&str; 3] = ["activity-log.json", "todos.json", "archive.json"];
const PENDING: &str = "catch-import-pending.json";
#[derive(Clone, PartialEq, Eq)]
pub struct Source(pub [Option<Vec<u8>>; 3]);
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub matters: usize,
    pub sessions: usize,
    pub todos: usize,
    pub archived: usize,
    pub trash: usize,
}
#[derive(Serialize, Deserialize)]
struct Change {
    file: String,
    before: Option<String>,
    after: String,
    original: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct Plan {
    version: u32,
    id: String,
    changes: Vec<Change>,
}
fn string(bytes: Option<Vec<u8>>) -> Result<Option<String>> {
    bytes
        .map(|value| String::from_utf8(value).map_err(|_| "导入文件须为 UTF-8".into()))
        .transpose()
}
fn rows(bytes: Option<&[u8]>) -> Result<Vec<todos::Todo>> {
    let rows: Vec<todos::Todo> = bytes
        .map(serde_json::from_slice)
        .transpose()
        .map_err(|error| format!("待办文件无法读取：{error}"))?
        .unwrap_or_default();
    todos::validate_records(&rows)?;
    if rows.iter().any(|row| row.recurrence.is_some()) {
        return Err("此入口只导入旧版数据，周期数据请使用完整备份".into());
    }
    Ok(rows)
}
fn decode(source: &Source, now: i64) -> Result<(tracking::Document, todos::Document)> {
    let tracking = source.0[0]
        .as_deref()
        .map(decode_import)
        .transpose()?
        .unwrap_or_default()
        .migrated()?
        .recover_interrupted(now)?;
    let todos = todos::Document {
        todos: rows(source.0[1].as_deref())?,
        archive: rows(source.0[2].as_deref())?,
        revision: 0,
    };
    // Legacy archives can contain the same completed row as the live file.
    // Preserve both source files; normal history projection deduplicates by ID.
    Ok((tracking, todos))
}
pub fn inspect(source: &Source, now: i64) -> Result<Summary> {
    let (tracking, todos) = decode(source, now)?;
    let summary = Summary {
        matters: tracking.activities.len(),
        sessions: tracking
            .activities
            .iter()
            .map(|row| row.sessions.as_ref().map_or(0, Vec::len))
            .sum(),
        todos: todos.todos.len(),
        archived: todos.archive.len(),
        trash: tracking.trash.len(),
    };
    if summary.matters + summary.todos + summary.archived + summary.trash == 0 {
        return Err("此目录中没有可导入的旧版记录".into());
    }
    Ok(summary)
}
pub fn prepare(
    disk: &dyn Disk,
    source: &Source,
    now: i64,
    id: &str,
    revision_floor: u64,
) -> Result<Plan> {
    inspect(source, now)?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("导入标识无效".into());
    }
    for name in [
        PENDING,
        "catch-archive-pending.json",
        "catch-schedule-pending.json",
    ] {
        if disk.read(name)?.is_some() {
            return Err("请先重启以恢复未完成的数据操作".into());
        }
    }
    let current = Source([
        disk.read(FILES[0])?,
        disk.read(FILES[1])?,
        disk.read(FILES[2])?,
    ]);
    let (before_tracking, before_todos) = decode(&current, now)?;
    if !before_tracking.activities.is_empty()
        || !before_tracking.trash.is_empty()
        || !before_todos.todos.is_empty()
        || !before_todos.archive.is_empty()
    {
        return Err("旧版导入仅适用于空的新数据目录，现有记录不会被覆盖".into());
    }
    for (index, name) in FILES.iter().enumerate() {
        if current.0[index].is_none() && disk.read(&format!("{name}.bak"))?.is_some() {
            return Err("主文件缺失但备份存在，请先恢复文件".into());
        }
    }
    if let Some(bytes) = disk.read("catch-recurrences.json")? {
        let rules: catch_core::recurrence::Document =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        rules.validate()?;
        if !rules.templates.is_empty() {
            return Err("请使用没有周期规则的新数据目录导入".into());
        }
    }
    let (mut tracking, todos) = decode(source, now)?;
    tracking.revision = tracking
        .revision
        .max(revision_floor)
        .checked_add(1)
        .ok_or("记录版本超出范围")?;
    let after = [
        encode(&tracking)?,
        encode(&todos.todos)?,
        encode(&todos.archive)?,
    ];
    let changes = FILES
        .iter()
        .enumerate()
        .map(|(index, file)| {
            Ok(Change {
                file: (*file).into(),
                before: string(current.0[index].clone())?,
                after: string(Some(after[index].clone()))?.unwrap(),
                original: string(source.0[index].clone())?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Plan {
        version: 1,
        id: id.into(),
        changes,
    })
}
fn validate(plan: &Plan) -> Result<()> {
    if plan.version != 1
        || plan.id.is_empty()
        || !plan
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        || plan.changes.len() != 3
        || !FILES
            .iter()
            .all(|file| plan.changes.iter().filter(|c| c.file == *file).count() == 1)
    {
        return Err("导入恢复日志无效，请保留文件".into());
    }
    let get = |file: &str| {
        plan.changes
            .iter()
            .find(|c| c.file == file)
            .unwrap()
            .after
            .as_bytes()
    };
    decode_import(get(FILES[0]))?;
    rows(Some(get(FILES[1])))?;
    rows(Some(get(FILES[2])))?;
    Ok(())
}
pub fn recover(disk: &dyn Disk) -> Result<()> {
    let Some(bytes) = disk.read(PENDING)? else {
        return Ok(());
    };
    for name in ["catch-archive-pending.json", "catch-schedule-pending.json"] {
        if disk.read(name)?.is_some() {
            return Err("存在冲突的数据恢复日志，请保留文件检查".into());
        }
    }
    let plan: Plan =
        serde_json::from_slice(&bytes).map_err(|e| format!("导入恢复日志无法读取：{e}"))?;
    validate(&plan)?;
    for change in &plan.changes {
        let current = string(disk.read(&change.file)?)?;
        if current != change.before && current.as_deref() != Some(change.after.as_str()) {
            return Err("导入期间数据被外部修改，已停止恢复".into());
        }
    }
    // Receipt contains byte-exact source originals and the previous target.
    // It is durable before any profile file is replaced.
    let receipt = format!("catch-import-{}.json", plan.id);
    match disk.read(&receipt)? {
        Some(saved) if saved != bytes => return Err("导入备份标识冲突".into()),
        None => disk.write_atomic(&receipt, &bytes)?,
        _ => {}
    }
    for change in &plan.changes {
        if disk.read(&change.file)?.as_deref() != Some(change.after.as_bytes()) {
            disk.write_atomic(&change.file, change.after.as_bytes())?;
        }
    }
    disk.remove(PENDING)
}
pub fn commit(disk: &dyn Disk, plan: &Plan) -> Result<()> {
    validate(plan)?;
    if disk.read(PENDING)?.is_some() {
        return Err("已有未完成导入，请重启恢复".into());
    }
    for change in &plan.changes {
        if string(disk.read(&change.file)?)? != change.before {
            return Err("预览后新版本记录已变化，请重新预览".into());
        }
    }
    disk.write_atomic(PENDING, &encode(plan)?)?;
    recover(disk)
}
