use crate::{encode, Disk, Result};
use catch_core::todos::{validate_records, Command, Context, Document, Todo};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
const TODOS: &str = "todos.json";
const ARCHIVE: &str = "archive.json";
const PENDING: &str = "catch-archive-pending.json";
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Journal {
    version: u32,
    before: Vec<Todo>,
    after: Vec<Todo>,
    archive_before: Vec<Todo>,
    archive_after: Vec<Todo>,
}
fn read(disk: &dyn Disk, key: &str) -> Result<Vec<Todo>> {
    let Some(bytes) = disk.read(key)? else {
        if disk.read(&format!("{key}.bak"))?.is_some() {
            return Err(format!("{key} 缺失但备份存在，已停止写入，请先恢复文件"));
        }
        return Ok(vec![]);
    };
    let rows: Vec<Todo> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("待办文件无法读取，原件已保留：{error}"))?;
    validate_records(&rows)?;
    Ok(rows)
}
fn replace(disk: &dyn Disk, key: &str, rows: &[Todo]) -> Result<()> {
    if let Some(bytes) = disk.read(key)? {
        disk.write_atomic(&format!("{key}.bak"), &bytes)?;
    }
    disk.write_atomic(key, &encode(&rows)?)
}
fn recover(disk: &dyn Disk) -> Result<()> {
    let Some(bytes) = disk.read(PENDING)? else {
        return Ok(());
    };
    let journal: Journal =
        serde_json::from_slice(&bytes).map_err(|error| format!("归档恢复日志无效：{error}"))?;
    if journal.version != 1 {
        return Err("归档恢复版本无效，请保留文件".into());
    }
    for rows in [
        &journal.before,
        &journal.after,
        &journal.archive_before,
        &journal.archive_after,
    ] {
        validate_records(rows)?;
    }
    let todos = read(disk, TODOS)?;
    let archive = read(disk, ARCHIVE)?;
    if (todos != journal.before && todos != journal.after)
        || (archive != journal.archive_before && archive != journal.archive_after)
    {
        return Err("归档期间数据被外部修改，已停止恢复".into());
    }
    if archive != journal.archive_after {
        replace(disk, ARCHIVE, &journal.archive_after)?;
    }
    if todos != journal.after {
        replace(disk, TODOS, &journal.after)?;
    }
    disk.remove(PENDING)
}
pub struct TodosStore {
    disk: Arc<dyn Disk>,
    state: Document,
    expected: Option<Vec<u8>>,
    expected_archive: Option<Vec<u8>>,
    blocked: bool,
}
impl TodosStore {
    pub fn open(disk: Arc<dyn Disk>) -> Result<Self> {
        recover(disk.as_ref())?;
        let state = Document {
            todos: read(disk.as_ref(), TODOS)?,
            archive: read(disk.as_ref(), ARCHIVE)?,
            revision: 0,
        };
        let expected = disk.read(TODOS)?;
        let expected_archive = disk.read(ARCHIVE)?;
        Ok(Self {
            disk,
            state,
            expected,
            expected_archive,
            blocked: false,
        })
    }
    pub fn snapshot(&self) -> &Document {
        &self.state
    }
    pub fn command(
        &mut self,
        command: &Command,
        revision: u64,
        context: Context<'_>,
    ) -> Result<Document> {
        if self.blocked {
            return Err("存储状态需要恢复，已停止写入".into());
        }
        let candidate = self.state.apply(command, revision, context)?;
        if self.disk.read(TODOS)? != self.expected
            || self.disk.read(ARCHIVE)? != self.expected_archive
        {
            self.blocked = true;
            return Err("待办文件被外部修改，已停止写入".into());
        }
        let result = if matches!(command, Command::Archive) {
            let journal = Journal {
                version: 1,
                before: self.state.todos.clone(),
                after: candidate.todos.clone(),
                archive_before: self.state.archive.clone(),
                archive_after: candidate.archive.clone(),
            };
            self.disk
                .write_atomic(PENDING, &encode(&journal)?)
                .and_then(|_| recover(self.disk.as_ref()))
        } else {
            replace(self.disk.as_ref(), TODOS, &candidate.todos)
        };
        if let Err(error) = result {
            // A partially applied journal must be recovered and reloaded before
            // any further command. Preserve the original error for the caller.
            match Self::open(self.disk.clone()) {
                Ok(mut recovered) => {
                    recovered.state.revision = self
                        .state
                        .revision
                        .checked_add(1)
                        .ok_or("待办版本超出范围")?;
                    *self = recovered;
                }
                Err(_) => self.blocked = true,
            }
            return Err(format!("保存未确认，请核对记录后重试：{error}"));
        }
        self.state = candidate;
        self.expected = self.disk.read(TODOS)?;
        self.expected_archive = self.disk.read(ARCHIVE)?;
        Ok(self.state.clone())
    }
}
