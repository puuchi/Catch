use crate::{encode, Disk, Result};
use catch_core::growth::{Command, Context, Document};
use std::sync::Arc;
const FILE: &str = "catch-growth.json";
/// Daily choices are committed here before a success reaches any window.
pub struct GrowthStore {
    disk: Arc<dyn Disk>,
    state: Document,
    expected: Option<Vec<u8>>,
    blocked: bool,
}
impl GrowthStore {
    pub fn open(disk: Arc<dyn Disk>) -> Result<Self> {
        let expected = disk.read(FILE)?;
        let state = if let Some(bytes) = &expected {
            serde_json::from_slice(bytes)
                .map_err(|error| format!("成长数据无法读取，已停止写入保护原件：{error}"))?
        } else {
            if disk.read("catch-growth.json.bak")?.is_some() {
                return Err("成长主文件缺失但备份存在，请先恢复文件".into());
            }
            Document::default()
        };
        state.validate()?;
        Ok(Self {
            disk,
            state,
            expected,
            blocked: false,
        })
    }
    pub fn snapshot(&self) -> &Document {
        &self.state
    }
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }
    fn save(&mut self, candidate: Document) -> Result<Document> {
        if self.blocked {
            return Err("成长存储状态需要恢复，请检查文件并重启".into());
        }
        if candidate == self.state {
            return Ok(self.state.clone());
        }
        candidate.validate()?;
        if self.disk.read(FILE)? != self.expected {
            self.blocked = true;
            return Err("成长文件被外部修改，已停止写入".into());
        }
        let bytes = encode(&candidate)?;
        if let Some(old) = &self.expected {
            self.disk.write_atomic("catch-growth.json.bak", old)?;
        }
        if let Err(error) = self.disk.write_atomic(FILE, &bytes) {
            if self.disk.read(FILE).ok() != Some(self.expected.clone()) {
                self.blocked = true;
            }
            return Err(error);
        }
        self.expected = Some(bytes);
        self.state = candidate;
        Ok(self.state.clone())
    }
    pub fn command(
        &mut self,
        command: &Command,
        revision: u64,
        context: Context<'_>,
    ) -> Result<Document> {
        self.save(self.state.apply(command, revision, context)?)
    }
    pub fn observe(&mut self, context: Context<'_>, invested_ms: i64) -> Result<Document> {
        self.save(self.state.observe(context, invested_ms)?)
    }
}
