use crate::{encode, Disk, Result};
use catch_core::tracking::{timestamp, Command, Context, Document};
use std::sync::Arc;
const FILE: &str = "activity-log.json";
const BACKUP: &str = "activity-log.json.bak";
pub struct TrackingStore {
    disk: Arc<dyn Disk>,
    state: Document,
    expected: Option<Vec<u8>>,
    blocked: bool,
    pub notice: Option<String>,
}
enum ReadError {
    Future(String),
    Invalid(String),
}
fn decode(bytes: &[u8]) -> std::result::Result<Document, ReadError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| ReadError::Invalid(error.to_string()))?;
    if !matches!(value["version"].as_u64(), Some(1 | 2))
        || value
            .get("sessionVersion")
            .is_some_and(|version| version.as_u64() != Some(1))
    {
        return Err(ReadError::Future(
            "无法识别活动数据版本，请保留文件并使用兼容版本".into(),
        ));
    }
    if value
        .get("trash")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| !matches!(entry["kind"].as_str(), Some("matter" | "session")))
        })
    {
        return Err(ReadError::Future(
            "无法识别活动回收站版本，请保留文件并使用兼容版本".into(),
        ));
    }
    let state: Document = serde_json::from_value(value).map_err(|error| {
        if error.to_string().contains("unknown variant") {
            ReadError::Future(format!("无法识别数据中的类型版本，原件已保留：{error}"))
        } else {
            ReadError::Invalid(error.to_string())
        }
    })?;
    state.validate().map_err(ReadError::Invalid)?;
    Ok(state)
}
pub(crate) fn decode_import(bytes: &[u8]) -> Result<Document> {
    decode(bytes).map_err(|error| match error {
        ReadError::Future(message) | ReadError::Invalid(message) => message,
    })
}
impl TrackingStore {
    pub fn open(disk: Arc<dyn Disk>, now: i64, recovery_id: &str) -> Result<Self> {
        if recovery_id.is_empty()
            || !recovery_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err("恢复标识无效".into());
        }
        let bytes = disk.read(FILE)?;
        let mut trusted_bytes = bytes.clone();
        let mut notice = None;
        let loaded = match bytes.as_deref() {
            None => {
                if disk.read(BACKUP)?.is_some() {
                    return Err("活动主文件缺失但备份存在，已停止写入，请先检查并恢复文件".into());
                }
                Document::default()
            }
            Some(bytes) => match decode(bytes) {
                Ok(state) => state,
                Err(ReadError::Future(message)) => return Err(message),
                Err(ReadError::Invalid(cause)) => {
                    let backup = disk
                        .read(BACKUP)?
                        .ok_or_else(|| format!("活动数据读取失败，已停止写入保护原件：{cause}"))?;
                    let recovered = decode(&backup)
                        .map_err(|_| "主文件和备份均无法读取，已停止写入保护原件")?;
                    trusted_bytes = Some(backup);
                    disk.write_atomic(&format!("activity-log.json.damaged-{recovery_id}"), bytes)?;
                    notice = Some("活动文件损坏，已恢复上一份备份；损坏原件已保留".into());
                    recovered
                }
            },
        };
        let migrated = loaded.migrated()?;
        if migrated != loaded {
            // Save original bytes, including fields predating the typed model.
            disk.write_atomic(
                &format!(
                    "activity-log.json.v{}-backup-{recovery_id}.json",
                    loaded.version
                ),
                trusted_bytes.as_deref().ok_or("缺少迁移原件")?,
            )?;
        }
        let state = migrated.recover_interrupted(now)?;
        let recovered = state != migrated;
        if recovered {
            notice = Some(
                [
                    notice.as_deref().unwrap_or(""),
                    "上次运行意外中断，已恢复到检查点并暂停",
                ]
                .join(" ")
                .trim()
                .into(),
            );
        }
        let mut store = Self {
            disk,
            state,
            expected: bytes,
            blocked: false,
            notice,
        };
        if store.state != loaded || store.notice.is_some() {
            let encoded = encode(&store.state)?;
            // Do not replace a valid backup with a corrupt main file.
            if store.disk.read(FILE)? != store.expected {
                return Err("恢复期间活动文件被外部修改，已停止写入".into());
            }
            store.disk.write_atomic(FILE, &encoded)?;
            store.expected = Some(encoded);
        }
        Ok(store)
    }
    pub fn snapshot(&self) -> &Document {
        &self.state
    }
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }
    fn save(&mut self, candidate: Document) -> Result<()> {
        if self.blocked {
            return Err("存储状态需要恢复，已停止写入，请检查文件并重启".into());
        }
        candidate.validate()?;
        if self.disk.read(FILE)? != self.expected {
            self.blocked = true;
            return Err("活动文件被外部修改，已停止写入".into());
        }
        let encoded = encode(&candidate)?;
        if let Some(previous) = &self.expected {
            self.disk.write_atomic(BACKUP, previous)?;
        }
        if let Err(error) = self.disk.write_atomic(FILE, &encoded) {
            if self.disk.read(FILE).ok() != Some(self.expected.clone()) {
                self.blocked = true;
            }
            return Err(error);
        }
        self.expected = Some(encoded);
        self.state = candidate;
        Ok(())
    }
    pub fn command(
        &mut self,
        command: &Command,
        revision: u64,
        context: Context<'_>,
    ) -> Result<Document> {
        if self.blocked {
            return Err("暂停状态尚未确认保存，请检查磁盘后重启".into());
        }
        let candidate = self.state.apply(command, revision, context)?;
        if candidate != self.state {
            self.save(candidate)?;
        }
        Ok(self.state.clone())
    }
    pub fn interrupt(&mut self, now: i64, reason: &str) -> Result<Document> {
        let candidate = self.state.apply(
            &Command::Pause {
                reason: reason.into(),
                cutoff: None,
            },
            self.state.revision,
            Context {
                now,
                new_id: "unused",
            },
        )?;
        if candidate == self.state {
            return Ok(self.state.clone());
        }
        if let Err(error) = self.save(candidate.clone()) {
            // A system sleep cannot be cancelled by a full disk. Stop memory
            // timing immediately; forbid subsequent mutations until recovery.
            self.state = candidate;
            self.blocked = true;
            self.notice = Some(format!("暂停状态尚未保存：{error}"));
            return Err(error);
        }
        Ok(self.state.clone())
    }
    pub fn checkpoint(&mut self, now: i64) -> Result<Document> {
        if self.blocked {
            return Err("暂停状态尚未确认保存，请检查磁盘后重启".into());
        }
        if self.state.current().is_none() {
            return Ok(self.state.clone());
        }
        let previous = self
            .state
            .checkpoint_at
            .as_deref()
            .map(timestamp)
            .transpose()?
            .unwrap_or(now);
        if now < previous || now.saturating_sub(previous) > 90_000 {
            return self.command(
                &Command::Pause {
                    reason: "clock-gap".into(),
                    cutoff: Some(previous),
                },
                self.state.revision,
                Context {
                    now,
                    new_id: "unused",
                },
            );
        }
        self.command(
            &Command::Checkpoint,
            self.state.revision,
            Context {
                now,
                new_id: "unused",
            },
        )
    }
}
