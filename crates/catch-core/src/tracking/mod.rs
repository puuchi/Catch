mod color;
mod commands;
mod validation;

use crate::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub use commands::{Command, Context};
type Extensions = BTreeMap<String, Value>;

pub fn timestamp(value: &str) -> Result<i64> {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.timestamp_millis())
        .map_err(|_| "时间格式无效".into())
}
pub fn iso(time: i64) -> Result<String> {
    DateTime::<Utc>::from_timestamp_millis(time)
        .map(|time| time.to_rfc3339_opts(SecondsFormat::Millis, true))
        .ok_or_else(|| "时间超出范围".into())
}
pub(crate) fn text_length(value: &str) -> usize {
    value.encode_utf16().count()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Running,
    Paused,
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub start: String,
    pub end: Option<String>,
    #[serde(flatten)]
    pub extra: Extensions,
}
impl Segment {
    pub fn new(start: &str, end: Option<&str>) -> Self {
        Self {
            start: start.into(),
            end: end.map(Into::into),
            extra: Extensions::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub progress: String,
    pub segments: Vec<Segment>,
    #[serde(flatten)]
    pub extra: Extensions,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Matter {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub status: Status,
    pub segments: Vec<Segment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Vec<Session>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_session_index: Option<u64>,
    pub todo_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub pause_reason: Option<String>,
    #[serde(flatten)]
    pub extra: Extensions,
}
impl Matter {
    pub fn ensure_sessions(&mut self) -> Result<()> {
        if self.sessions.is_none() {
            self.sessions = Some(if self.segments.is_empty() {
                vec![Session {
                    id: format!("{}:0", self.id),
                    start_time: self.start_time.clone(),
                    end_time: Some(
                        self.end_time
                            .clone()
                            .unwrap_or_else(|| self.start_time.clone()),
                    ),
                    progress: String::new(),
                    segments: vec![],
                    extra: Extensions::new(),
                }]
            } else {
                self.segments
                    .iter()
                    .enumerate()
                    .map(|(index, segment)| Session {
                        id: format!("{}:{index}", self.id),
                        start_time: segment.start.clone(),
                        end_time: segment.end.clone(),
                        progress: String::new(),
                        segments: vec![segment.clone()],
                        extra: Extensions::new(),
                    })
                    .collect()
            });
        }
        if self.next_session_index.is_none() {
            let sessions = self.sessions.as_ref().unwrap();
            let mut next = sessions.len() as u64;
            for session in sessions {
                if let Some(index) = session
                    .id
                    .strip_prefix(&format!("{}:", self.id))
                    .and_then(|value| value.parse::<u64>().ok())
                {
                    next = next.max(index.checked_add(1).ok_or("投入序号超出范围")?);
                }
            }
            self.next_session_index = Some(next);
        }
        Ok(())
    }
    pub(crate) fn sync_segments(&mut self) {
        self.segments = self
            .sessions
            .as_ref()
            .unwrap()
            .iter()
            .flat_map(|session| session.segments.clone())
            .collect();
        self.segments
            .sort_by_key(|segment| timestamp(&segment.start).unwrap_or(i64::MAX));
    }
    pub(crate) fn append(&mut self, time: &str) -> Result<()> {
        self.ensure_sessions()?;
        let index = self.next_session_index.unwrap();
        self.next_session_index = Some(index.checked_add(1).ok_or("投入序号超出范围")?);
        self.sessions.as_mut().unwrap().push(Session {
            id: format!("{}:{index}", self.id),
            start_time: time.into(),
            end_time: None,
            progress: String::new(),
            segments: vec![Segment::new(time, None)],
            extra: Extensions::new(),
        });
        self.sync_segments();
        Ok(())
    }
    pub(crate) fn close(&mut self, now: i64) -> Result<()> {
        self.ensure_sessions()?;
        for session in self.sessions.as_mut().unwrap() {
            if let Some(segment) = session.segments.last_mut() {
                if segment.end.is_none() {
                    let end = iso(now.max(timestamp(&segment.start)?))?;
                    segment.end = Some(end.clone());
                    session.end_time = Some(end);
                }
            }
        }
        self.sync_segments();
        Ok(())
    }
    pub(crate) fn pause(&mut self, now: i64, reason: &str) -> Result<()> {
        self.close(now)?;
        self.status = Status::Paused;
        self.pause_reason = Some(reason.into());
        self.updated_at = iso(now)?;
        Ok(())
    }
    pub(crate) fn finish(&mut self, now: i64) -> Result<()> {
        self.close(now)?;
        let last = self
            .segments
            .last()
            .and_then(|segment| segment.end.as_deref())
            .map(timestamp)
            .transpose()?
            .unwrap_or(0);
        self.end_time = Some(iso(now.max(timestamp(&self.start_time)?).max(last))?);
        self.status = Status::Completed;
        self.pause_reason = None;
        self.updated_at = self.end_time.clone().unwrap();
        Ok(())
    }
    /// Clips effective intervals to supplied local-day boundaries. The caller
    /// resolves timezone/DST; no fixed 24-hour assumption lives in the core.
    pub fn duration(&self, now: i64, bounds: Option<(i64, i64)>) -> Result<i64> {
        let (from, to) = bounds.unwrap_or((i64::MIN, i64::MAX));
        if to < from {
            return Err("日期范围无效".into());
        }
        self.segments.iter().try_fold(0_i64, |total, segment| {
            let start = timestamp(&segment.start)?.max(from);
            let end = segment
                .end
                .as_deref()
                .map(timestamp)
                .transpose()?
                .unwrap_or(now)
                .min(to);
            total
                .checked_add(end.saturating_sub(start).max(0))
                .ok_or_else(|| "累计时长超出范围".into())
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrashKind {
    Matter,
    Session,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    pub id: String,
    pub kind: TrashKind,
    pub deleted_at: String,
    pub activity: Matter,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Session>,
    #[serde(flatten)]
    pub extra: Extensions,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_version: Option<u32>,
    #[serde(default)]
    pub revision: u64,
    pub checkpoint_at: Option<String>,
    pub activities: Vec<Matter>,
    #[serde(default)]
    pub trash: Vec<TrashEntry>,
    #[serde(flatten)]
    pub extra: Extensions,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 2,
            session_version: Some(1),
            revision: 0,
            checkpoint_at: None,
            activities: vec![],
            trash: vec![],
            extra: Extensions::new(),
        }
    }
}
impl Document {
    pub fn current(&self) -> Option<&Matter> {
        self.activities
            .iter()
            .find(|matter| matter.status == Status::Running)
    }
    /// Pure migration candidate. Storage must back up the original bytes before
    /// persisting this result. Unknown extension fields survive round trips.
    pub fn migrated(&self) -> Result<Self> {
        self.validate()?;
        let mut next = self.clone();
        next.version = 2;
        next.session_version = Some(1);
        for index in 0..next.activities.len() {
            next.activities[index].ensure_sessions()?;
            if next.activities[index].color.is_none() {
                let color = color::next_color(&next);
                next.activities[index].color = Some(color);
            }
        }
        next.validate()?;
        Ok(next)
    }
    pub fn recover_interrupted(&self, now: i64) -> Result<Self> {
        let mut next = self.clone();
        let checkpoint = next.checkpoint_at.as_deref().map(timestamp).transpose()?;
        if let Some(current) = next
            .activities
            .iter_mut()
            .find(|matter| matter.status == Status::Running)
        {
            let start = timestamp(&current.segments.last().ok_or("缺少开放时间段")?.start)?;
            let cutoff = start.max(checkpoint.unwrap_or(start).min(now));
            current.pause(cutoff, "interrupted")?;
        }
        next.validate()?;
        Ok(next)
    }
}
