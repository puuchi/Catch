//! Small roll-forward transaction for finishing tracking and its linked todo.
//! The durable intent includes recurrence effects and the replay receipt. No
//! reader may open either repository before recovery; unresolved I/O blocks both.
use crate::{encode, Disk, Result, TodosStore, TrackingStore};
use catch_core::{recurrence, todos, tracking};
use serde::{Deserialize, Serialize};
mod receipts;
use receipts::FILE as RECEIPTS;
pub use receipts::{Receipt, ReceiptStore, MAX_RESPONSES, RESPONSE_BUDGET};

const PENDING: &str = "catch-finish-pending.json";
const FILES: [&str; 4] = [
    "activity-log.json",
    "todos.json",
    "catch-recurrences.json",
    RECEIPTS,
];
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum FileContents {
    Text(String),
    Legacy(Vec<u8>),
}
impl FileContents {
    fn new(bytes: Vec<u8>) -> Result<Self> {
        String::from_utf8(bytes)
            .map(Self::Text)
            .map_err(|_| "结束事务文件不是 UTF-8".into())
    }
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Legacy(bytes) => bytes,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Change {
    file: String,
    before: Option<FileContents>,
    after: FileContents,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    version: u32,
    changes: Vec<Change>,
}
pub fn lookup(disk: &dyn Disk, request_id: &str) -> Result<Option<Receipt>> {
    Ok(receipts::decode(receipts::read(disk)?.as_deref())?
        .0
        .into_iter()
        .find(|row| row.request_id == request_id))
}
fn validate(journal: &Journal) -> Result<()> {
    if !matches!(journal.version, 1 | 2)
        || journal.changes.len() != FILES.len()
        || FILES
            .iter()
            .any(|file| journal.changes.iter().filter(|c| c.file == *file).count() != 1)
    {
        return Err("结束恢复日志版本或文件集合无效，请保留文件".into());
    }
    for after in [false, true] {
        let get = |file: &str| -> Option<&[u8]> {
            let c = journal.changes.iter().find(|c| c.file == file).unwrap();
            if after {
                Some(c.after.bytes())
            } else {
                c.before.as_ref().map(FileContents::bytes)
            }
        };
        if let Some(raw) = get(FILES[0]) {
            crate::tracking::decode_import(raw)?;
        }
        let rows: Vec<todos::Todo> = get(FILES[1])
            .map(serde_json::from_slice)
            .transpose()
            .map_err(|e| format!("结束事务待办无效：{e}"))?
            .unwrap_or_default();
        todos::validate_records(&rows)?;
        let rules: recurrence::Document = get(FILES[2])
            .map(serde_json::from_slice)
            .transpose()
            .map_err(|e| format!("结束事务周期无效：{e}"))?
            .unwrap_or_default();
        rules.validate_links(&todos::Document {
            todos: rows,
            archive: vec![],
            revision: 0,
        })?;
        receipts::decode(get(RECEIPTS))?;
    }
    Ok(())
}
pub fn recover(disk: &dyn Disk) -> Result<()> {
    let Some(bytes) = disk.read(PENDING)? else {
        return Ok(());
    };
    for other in [
        "catch-import-pending.json",
        "catch-archive-pending.json",
        "catch-schedule-pending.json",
    ] {
        if disk.read(other)?.is_some() {
            return Err("存在多份未完成事务，已停止结束恢复，请保留文件".into());
        }
    }
    let journal: Journal =
        serde_json::from_slice(&bytes).map_err(|e| format!("结束恢复日志无效：{e}"))?;
    validate(&journal)?;
    // Check every preimage before changing anything. Unknown external changes
    // never get overwritten, and intent remains available for manual recovery.
    for c in &journal.changes {
        let current = disk.read(&c.file)?;
        if current.as_deref() != c.before.as_ref().map(FileContents::bytes)
            && current.as_deref() != Some(c.after.bytes())
        {
            return Err("结束事务期间文件被外部修改，已停止恢复".into());
        }
    }
    for c in &journal.changes {
        if disk.read(&c.file)?.as_deref() == Some(c.after.bytes()) {
            continue;
        }
        if let Some(before) = &c.before {
            disk.write_atomic(&format!("{}.bak", c.file), before.bytes())?;
        }
        disk.write_atomic(&c.file, c.after.bytes())?;
    }
    disk.remove(PENDING)
}

pub struct Candidate {
    pub tracking: tracking::Document,
    pub todos: todos::Document,
    pub rules: recurrence::Document,
}
pub fn commit(
    tracking: &mut TrackingStore,
    todos: &mut TodosStore,
    candidate: Candidate,
    receipt: Receipt,
    receipts: &mut ReceiptStore,
) -> Result<()> {
    if tracking.blocked || todos.blocked {
        return Err("存储状态需要恢复，已停止写入".into());
    }
    if !std::sync::Arc::ptr_eq(&tracking.disk, &todos.disk)
        || !std::sync::Arc::ptr_eq(&tracking.disk, &receipts.disk)
    {
        return Err("结束事务必须使用同一存储".into());
    }
    todos.check_external()?;
    let disk = tracking.disk.clone();
    if disk.read(PENDING)?.is_some() || disk.read(FILES[0])? != tracking.expected {
        tracking.blocked = true;
        todos.blocked = true;
        return Err("发现未完成结束事务或活动文件外部修改，请重启恢复".into());
    }
    candidate.tracking.validate()?;
    candidate.rules.validate_links(&candidate.todos)?;
    if candidate.todos.archive != todos.state.archive {
        return Err("结束事务不得修改归档".into());
    }
    let on_disk = disk.read(RECEIPTS)?;
    if on_disk != receipts.expected || (on_disk.is_none() && disk.read(receipts::BACKUP)?.is_some())
    {
        tracking.blocked = true;
        todos.blocked = true;
        return Err("结束回执文件被外部修改，已停止写入，请重启核对".into());
    }
    let (saved, encoded_receipts) = receipts.prepare(receipt.clone())?;
    let encoded = [
        encode(&candidate.tracking)?,
        encode(&candidate.todos.todos)?,
        encode(&candidate.rules)?,
        encoded_receipts,
    ];
    let before = [
        tracking.expected.clone(),
        todos.expected.clone(),
        todos.expected_rules.clone(),
        receipts.expected.clone(),
    ];
    let journal = Journal {
        version: 2,
        changes: FILES
            .iter()
            .zip(before)
            .zip(encoded.iter())
            .map(|((file, before), after)| {
                Ok(Change {
                    file: (*file).into(),
                    before: before.map(FileContents::new).transpose()?,
                    after: FileContents::new(after.clone())?,
                })
            })
            .collect::<Result<Vec<_>>>()?,
    };
    validate(&journal)?;
    let intent = encode(&journal)?;
    // Until we can prove either no intent exists or recovery finished, neither
    // repository may accept writes using its now potentially stale snapshot.
    tracking.blocked = true;
    todos.blocked = true;
    let write = disk
        .write_atomic(PENDING, &intent)
        .and_then(|_| recover(disk.as_ref()));
    if let Err(error) = write {
        // Includes write-after-error and remove-after-error: replay the durable
        // intent, then check its receipt instead of trusting the error alone.
        recover(disk.as_ref())
            .map_err(|recovery| format!("结束尚未确认，请重启恢复：{error}；{recovery}"))?;
        if lookup(disk.as_ref(), &receipt.request_id)?.is_none() {
            // The intent write failed before changing any document.
            for c in &journal.changes {
                if disk.read(&c.file)?.as_deref() != c.before.as_ref().map(FileContents::bytes) {
                    return Err("结束写入状态不明，请重启恢复".into());
                }
            }
            tracking.blocked = false;
            todos.blocked = false;
            return Err(error);
        }
    }
    tracking.state = candidate.tracking;
    tracking.expected = Some(encoded[0].clone());
    todos.state = candidate.todos;
    todos.expected = Some(encoded[1].clone());
    todos.rules = candidate.rules;
    todos.expected_rules = Some(encoded[2].clone());
    receipts.accept(saved, Some(encoded[3].clone()));
    tracking.blocked = false;
    todos.blocked = false;
    Ok(())
}
