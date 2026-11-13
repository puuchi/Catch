use crate::tracking::{iso, text_length, timestamp};
use crate::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

pub fn calendar_date(value: &str) -> Result<NaiveDate> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| "日期无效")?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err("日期无效".into());
    }
    Ok(date)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: String,
    pub text: String,
    pub done: bool,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub urgent: bool,
    #[serde(default)]
    pub deadline: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub todos: Vec<Todo>,
    pub archive: Vec<Todo>,
    #[serde(default)]
    pub revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fields {
    pub text: String,
    #[serde(default)]
    pub notes: String,
    pub urgent: bool,
    #[serde(default)]
    pub deadline: Option<String>,
}
impl Fields {
    fn normalized(&self) -> Result<Self> {
        if self.text.trim().is_empty() || text_length(self.text.trim()) > 80 {
            return Err("待办内容须为 1–80 个字符".into());
        }
        if text_length(&self.notes) > 2000 {
            return Err("备注最多 2000 个字符".into());
        }
        if let Some(day) = &self.deadline {
            calendar_date(day)?;
        }
        Ok(Self {
            text: self.text.trim().into(),
            notes: self.notes.trim().into(),
            urgent: self.urgent,
            deadline: self.deadline.clone(),
        })
    }
    fn assign(&self, todo: &mut Todo) {
        todo.text = self.text.clone();
        todo.notes = self.notes.clone();
        todo.urgent = self.urgent;
        todo.deadline = self.deadline.clone();
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "kebab-case")]
pub enum Command {
    Create(Fields),
    Edit {
        id: String,
        #[serde(flatten)]
        fields: Fields,
    },
    Complete {
        id: String,
        done: bool,
    },
    Trash {
        id: String,
    },
    Restore {
        id: String,
    },
    Delete {
        id: String,
    },
    Archive,
}
pub struct Context<'a> {
    pub now: i64,
    pub local_day: &'a str,
    pub new_id: &'a str,
}

pub fn validate_records(records: &[Todo]) -> Result<()> {
    let mut ids = HashSet::new();
    for row in records {
        if row.id.is_empty() || !ids.insert(&row.id) {
            return Err("待办记录无效或 ID 重复".into());
        }
        if let Some(day) = &row.deadline {
            calendar_date(day)?;
        }
        if let Some(time) = &row.deleted_at {
            timestamp(time)?;
        }
        // Legacy history may contain a completion date without a full timestamp.
        if let Some(day) = &row.completed_at {
            calendar_date(day)?;
        }
    }
    Ok(())
}
impl Document {
    pub fn validate(&self) -> Result<()> {
        validate_records(&self.todos)?;
        validate_records(&self.archive)
    }
    fn index(&self, id: &str) -> Result<usize> {
        self.todos
            .iter()
            .position(|row| row.id == id)
            .ok_or_else(|| "待办已不存在".into())
    }
    pub fn history(&self) -> Vec<Todo> {
        let mut rows: Vec<Todo> = vec![];
        for row in self
            .archive
            .iter()
            .filter(|row| row.deleted_at.is_none())
            .chain(
                self.todos
                    .iter()
                    .filter(|row| row.done && row.deleted_at.is_none()),
            )
        {
            if let Some(index) = rows.iter().position(|existing| existing.id == row.id) {
                rows[index] = row.clone();
            } else {
                rows.push(row.clone());
            }
        }
        rows.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));
        rows
    }
    pub fn visible(&self, today: &str) -> Vec<&Todo> {
        let mut rows: Vec<_> = self
            .todos
            .iter()
            .filter(|row| {
                row.deleted_at.is_none()
                    && (!row.done || row.completed_at.as_deref() == Some(today))
            })
            .collect();
        rows.sort_by_key(|row| {
            if row.done {
                3
            } else if row.urgent {
                0
            } else if row.deadline.as_deref() == Some(today) {
                1
            } else {
                2
            }
        });
        rows
    }
    pub fn apply(
        &self,
        command: &Command,
        expected_revision: u64,
        context: Context<'_>,
    ) -> Result<Self> {
        if expected_revision != self.revision {
            return Err("待办已变化，请核对后重试；草稿已保留".into());
        }
        self.validate()?;
        calendar_date(context.local_day)?;
        let mut next = self.clone();
        match command {
            Command::Create(input) => {
                let fields = input.normalized()?;
                next.todos.insert(
                    0,
                    Todo {
                        id: context.new_id.into(),
                        text: fields.text,
                        notes: fields.notes,
                        urgent: fields.urgent,
                        deadline: fields.deadline,
                        done: false,
                        completed_at: None,
                        deleted_at: None,
                        extra: BTreeMap::new(),
                    },
                );
            }
            Command::Edit { id, fields } => {
                let fields = fields.normalized()?;
                let index = next.index(id)?;
                if next.todos[index].deleted_at.is_some() {
                    return Err("待办已删除".into());
                }
                fields.assign(&mut next.todos[index]);
            }
            Command::Complete { id, done } => {
                let index = next.index(id)?;
                let row = &mut next.todos[index];
                if row.deleted_at.is_some() {
                    return Err("完成操作无效".into());
                }
                row.done = *done;
                row.completed_at = if *done {
                    Some(context.local_day.into())
                } else {
                    None
                };
            }
            Command::Trash { id } => {
                let index = next.index(id)?;
                next.todos[index].deleted_at = Some(iso(context.now)?);
            }
            Command::Restore { id } => {
                let index = next.index(id)?;
                next.todos[index].deleted_at = None;
            }
            Command::Delete { id } => {
                let index = next.index(id)?;
                if next.todos[index].deleted_at.is_none() {
                    return Err("请先移入回收站".into());
                }
                next.todos.remove(index);
            }
            Command::Archive => {
                next.archive = next.history();
                next.todos
                    .retain(|row| !row.done || row.deleted_at.is_some());
            }
        }
        next.revision = self.revision.checked_add(1).ok_or("待办版本超出范围")?;
        next.validate()?;
        Ok(next)
    }
}
