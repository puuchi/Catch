//! A WebView may be discarded only after its complete UI draft is durable.
//! Generation checks reject callbacks from a destroyed or timed-out window.
use catch_storage::Disk;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

const FILE: &str = "catch-ui-state.json";
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Bubble,
    Opening,
    Expanded,
    Collapsing,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UiState {
    pub version: u32,
    #[serde(flatten)]
    pub body: serde_json::Map<String, Value>,
}
impl Default for UiState {
    fn default() -> Self {
        serde_json::from_value(json!({"version":1,"page":"todos","activityName":"","progressDrafts":{},"composer":{"text":"","notes":"","urgent":false,"deadline":null},"backgroundOpen":true})).unwrap()
    }
}
impl UiState {
    fn encoded(&self) -> Result<Vec<u8>, String> {
        if self.version != 1 {
            return Err("无法识别界面草稿版本，原件已保留".into());
        }
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        if bytes.len() > 256_000 {
            return Err("界面草稿过大，请先保存部分编辑".into());
        }
        Ok(bytes)
    }
}
pub struct Session {
    disk: Arc<dyn Disk>,
    saved: UiState,
    expected: Option<Vec<u8>>,
    phase: Phase,
    generation: u64,
}
impl Session {
    pub fn open(disk: Arc<dyn Disk>) -> Result<Self, String> {
        let expected = disk.read(FILE)?;
        let saved = match &expected {
            Some(bytes) => serde_json::from_slice(bytes)
                .map_err(|error| format!("界面草稿无法读取，原件已保留：{error}"))?,
            None => UiState::default(),
        };
        saved.encoded()?;
        Ok(Self {
            disk,
            saved,
            expected,
            phase: Phase::Bubble,
            generation: 0,
        })
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn saved(&self) -> &UiState {
        &self.saved
    }
    pub fn ensure(&self, generation: u64) -> Result<(), String> {
        if generation != self.generation || !matches!(self.phase, Phase::Opening | Phase::Expanded)
        {
            return Err("窗口已更新，请重新打开".into());
        }
        Ok(())
    }
    pub fn begin_open(&mut self) -> Result<Option<u64>, String> {
        if self.phase != Phase::Bubble {
            return Ok(None);
        }
        self.generation = self.generation.checked_add(1).ok_or("窗口版本超出范围")?;
        self.phase = Phase::Opening;
        Ok(Some(self.generation))
    }
    pub fn painted(&mut self, generation: u64) -> bool {
        if self.generation != generation || !matches!(self.phase, Phase::Opening | Phase::Expanded)
        {
            return false;
        }
        self.phase = Phase::Expanded;
        true
    }
    pub fn stash(&mut self, generation: u64, state: UiState) -> Result<(), String> {
        self.ensure(generation)?;
        let bytes = state.encoded()?;
        if self.disk.read(FILE)? != self.expected {
            return Err("界面草稿文件被外部修改，已停止覆盖".into());
        }
        if self.expected.as_deref() == Some(bytes.as_slice()) {
            self.saved = state;
            return Ok(());
        }
        if let Some(previous) = &self.expected {
            self.disk
                .write_atomic("catch-ui-state.json.bak", previous)?;
        }
        self.disk.write_atomic(FILE, &bytes)?;
        self.expected = Some(bytes);
        self.saved = state;
        Ok(())
    }
    pub fn prepare_collapse(&mut self, generation: u64, state: UiState) -> Result<(), String> {
        self.ensure(generation)?;
        if self.phase != Phase::Expanded {
            return Err("界面尚在加载，请稍后收起".into());
        }
        self.stash(generation, state)?;
        self.phase = Phase::Collapsing;
        Ok(())
    }
    pub fn finish_collapse(&mut self, generation: u64) -> bool {
        if self.generation != generation || self.phase != Phase::Collapsing {
            return false;
        }
        self.phase = Phase::Bubble;
        true
    }
    pub fn abort_collapse(&mut self, generation: u64) {
        if self.generation == generation && self.phase == Phase::Collapsing {
            self.phase = Phase::Expanded;
        }
    }
    pub fn failed_open(&mut self, generation: u64) -> bool {
        if self.generation != generation || self.phase != Phase::Opening {
            return false;
        }
        self.phase = Phase::Bubble;
        true
    }
}
