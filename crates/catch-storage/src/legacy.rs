//! Completes an import journal left by an earlier desktop build.
use crate::{tracking::decode_import, Disk, Result};
use catch_core::todos;
use serde::Deserialize;
const FILES: [&str; 3] = ["activity-log.json", "todos.json", "archive.json"];
const PENDING: &str = "catch-import-pending.json";
#[derive(Deserialize)]
struct Change {
    file: String,
    before: Option<String>,
    after: String,
}
#[derive(Deserialize)]
struct Plan {
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
