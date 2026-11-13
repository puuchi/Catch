use crate::{
    resources::{self, Resource},
    todos::{self, calendar_date, Fields, Todo},
    Result,
};
use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

fn after(date: NaiveDate, days: u64) -> Result<NaiveDate> {
    date.checked_add_days(Days::new(days))
        .ok_or_else(|| "周期日期超出范围".into())
}
fn before(date: NaiveDate, days: u64) -> Result<NaiveDate> {
    date.checked_sub_days(Days::new(days))
        .ok_or_else(|| "周期日期超出范围".into())
}
fn day_string(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}
fn month_day(year: i32, month: u32, day: u8) -> Result<NaiveDate> {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next = NaiveDate::from_ymd_opt(next_year, next_month, 1).ok_or("周期日期超出范围")?;
    let last = before(first_next, 1)?.day();
    NaiveDate::from_ymd_opt(year, month, u32::from(day).min(last))
        .ok_or_else(|| "周期日期超出范围".into())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Schedule {
    Daily,
    Weekly { weekdays: Vec<u8> },
    Every { days: u16 },
    Monthly { day: u8 },
}
impl Schedule {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Weekly { weekdays }
                if weekdays.is_empty()
                    || weekdays.iter().any(|day| *day > 6)
                    || weekdays.iter().collect::<HashSet<_>>().len() != weekdays.len() =>
            {
                Err("请选择不重复的星期，周一为 0、周日为 6".into())
            }
            Self::Every { days } if *days == 0 || *days > 3650 => Err("间隔须为 1–3650 天".into()),
            Self::Monthly { day } if !(1..=31).contains(day) => Err("每月日期须为 1–31".into()),
            _ => Ok(()),
        }
    }
    pub fn on_or_after(&self, anchor: NaiveDate, date: NaiveDate) -> Result<NaiveDate> {
        self.validate()?;
        let date = date.max(anchor);
        match self {
            Self::Daily => Ok(date),
            Self::Weekly { weekdays } => {
                for offset in 0..7 {
                    let candidate = after(date, offset)?;
                    if weekdays.contains(&(candidate.weekday().num_days_from_monday() as u8)) {
                        return Ok(candidate);
                    }
                }
                Err("每周周期无效".into())
            }
            Self::Every { days } => {
                let remainder = (date - anchor).num_days() % i64::from(*days);
                after(
                    date,
                    if remainder == 0 {
                        0
                    } else {
                        u64::from(*days) - remainder as u64
                    },
                )
            }
            Self::Monthly { day } => {
                let current = month_day(date.year(), date.month(), *day)?;
                if current >= date {
                    Ok(current)
                } else {
                    let (year, month) = if date.month() == 12 {
                        (date.year() + 1, 1)
                    } else {
                        (date.year(), date.month() + 1)
                    };
                    month_day(year, month, *day)
                }
            }
        }
    }
    pub fn latest(&self, anchor: NaiveDate, date: NaiveDate) -> Result<Option<NaiveDate>> {
        self.validate()?;
        if date < anchor {
            return Ok(None);
        }
        let candidate = match self {
            Self::Daily => date,
            Self::Weekly { weekdays } => {
                let offset = (0..7)
                    .find(|offset| {
                        before(date, *offset).is_ok_and(|candidate| {
                            weekdays.contains(&(candidate.weekday().num_days_from_monday() as u8))
                        })
                    })
                    .ok_or("每周周期无效")?;
                before(date, offset)?
            }
            Self::Every { days } => {
                before(date, ((date - anchor).num_days() % i64::from(*days)) as u64)?
            }
            Self::Monthly { day } => {
                let current = month_day(date.year(), date.month(), *day)?;
                if current <= date {
                    current
                } else {
                    let (year, month) = if date.month() == 1 {
                        (date.year() - 1, 12)
                    } else {
                        (date.year(), date.month() - 1)
                    };
                    month_day(year, month, *day)?
                }
            }
        };
        Ok((candidate >= anchor).then_some(candidate))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub template_id: String,
    pub scheduled_on: String,
}
impl Occurrence {
    pub fn validate(&self) -> Result<()> {
        if self.template_id.is_empty() {
            return Err("周期模板标识无效".into());
        }
        calendar_date(&self.scheduled_on)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub fields: Fields,
    pub resources: Vec<Resource>,
    pub schedule: Schedule,
    pub starts_on: String,
    pub next_on: String,
    pub last_emitted: Option<String>,
    pub stopped: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub version: u32,
    pub revision: u64,
    pub templates: Vec<Template>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            templates: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    Create {
        fields: Fields,
        #[serde(default)]
        resources: Vec<Resource>,
        schedule: Schedule,
        starts_on: String,
    },
    Update {
        id: String,
        fields: Fields,
        resources: Vec<Resource>,
        schedule: Schedule,
    },
    Stop {
        id: String,
    },
}
pub struct Candidate {
    pub rules: Document,
    pub todos: todos::Document,
}
impl Document {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err("无法识别周期数据版本，请保留文件".into());
        }
        let mut ids = HashSet::new();
        for template in &self.templates {
            if template.id.is_empty() || !ids.insert(&template.id) {
                return Err("周期模板标识无效或重复".into());
            }
            template.fields.normalized()?;
            resources::validate_list(&template.resources)?;
            template.schedule.validate()?;
            let start = calendar_date(&template.starts_on)?;
            let next = calendar_date(&template.next_on)?;
            if next < start {
                return Err("下次周期日期早于起点".into());
            }
            if template.schedule.on_or_after(start, next)? != next {
                return Err("下次周期日期与规则不一致".into());
            }
            if let Some(last) = &template.last_emitted {
                let last = calendar_date(last)?;
                if last < start || next <= last {
                    return Err("周期游标无效".into());
                }
            }
        }
        Ok(())
    }
    pub fn validate_links(&self, todos: &todos::Document) -> Result<()> {
        self.validate()?;
        todos.validate()?;
        let mut unfinished = HashSet::new();
        let mut occurrences = BTreeMap::new();
        for row in todos.todos.iter().chain(todos.archive.iter()) {
            if let Some(occurrence) = &row.recurrence {
                if !self
                    .templates
                    .iter()
                    .any(|template| template.id == occurrence.template_id)
                {
                    return Err("待办引用了不存在的周期模板，已停止写入".into());
                }
                if !row.done
                    && row.deleted_at.is_none()
                    && !unfinished.insert(&occurrence.template_id)
                {
                    return Err("这个周期已有一项未完成待办，请先完成或移除它".into());
                }
                // A legacy archive and live completed row may share one ID.
                let key = (&occurrence.template_id, &occurrence.scheduled_on);
                if let Some(existing) = occurrences.insert(key, &row.id) {
                    if existing != &row.id {
                        return Err("同一期周期实例重复".into());
                    }
                }
            }
        }
        Ok(())
    }
    fn bump(&mut self) -> Result<()> {
        self.revision = self.revision.checked_add(1).ok_or("周期版本超出范围")?;
        Ok(())
    }
    pub fn command(
        &self,
        command: &Command,
        todos: &todos::Document,
        revision: u64,
        today: &str,
        new_id: &str,
    ) -> Result<Candidate> {
        self.validate_links(todos)?;
        let today = calendar_date(today)?;
        if revision != self.revision {
            return Err("周期规则已变化，请核对后重试".into());
        }
        let mut rules = self.clone();
        let mut todos = todos.clone();
        match command {
            Command::Create {
                fields,
                resources,
                schedule,
                starts_on,
            } => {
                let fields = fields.normalized()?;
                resources::validate_list(resources)?;
                let start = calendar_date(starts_on)?;
                let next = schedule.on_or_after(start, start)?;
                rules.templates.push(Template {
                    id: new_id.into(),
                    fields,
                    resources: resources.clone(),
                    schedule: schedule.clone(),
                    starts_on: starts_on.clone(),
                    next_on: day_string(next),
                    last_emitted: None,
                    stopped: false,
                });
            }
            Command::Update {
                id,
                fields,
                resources,
                schedule,
            } => {
                let fields = fields.normalized()?;
                resources::validate_list(resources)?;
                let template = rules
                    .templates
                    .iter_mut()
                    .find(|template| template.id == *id && !template.stopped)
                    .ok_or("周期规则已停止或不存在")?;
                template.fields = fields.clone();
                template.resources = resources.clone();
                template.schedule = schedule.clone();
                let next =
                    schedule.on_or_after(calendar_date(&template.starts_on)?, after(today, 1)?)?;
                template.next_on = day_string(next);
                for row in &mut todos.todos {
                    if !row.done
                        && row.deleted_at.is_none()
                        && row
                            .recurrence
                            .as_ref()
                            .is_some_and(|occurrence| occurrence.template_id == *id)
                    {
                        row.text = fields.text.clone();
                        row.notes = fields.notes.clone();
                        row.urgent = fields.urgent;
                        row.resources = resources.clone();
                    }
                }
            }
            Command::Stop { id } => {
                let template = rules
                    .templates
                    .iter_mut()
                    .find(|template| template.id == *id)
                    .ok_or("周期规则已不存在")?;
                if template.stopped {
                    return Ok(Candidate { rules, todos });
                }
                template.stopped = true;
            }
        }
        rules.bump()?;
        let mut candidate = rules.reconcile(&todos, &day_string(today))?;
        // Any template edit changes the todos revision too: a renderer editing
        // one occurrence must detect an intervening "this and future" edit.
        if candidate.todos.revision == todos.revision {
            candidate.todos.revision = todos.revision.checked_add(1).ok_or("待办版本超出范围")?;
        }
        candidate.rules.validate_links(&candidate.todos)?;
        Ok(candidate)
    }
    pub fn reconcile(&self, todos: &todos::Document, today: &str) -> Result<Candidate> {
        self.validate_links(todos)?;
        let today = calendar_date(today)?;
        let mut rules = self.clone();
        let mut next = todos.clone();
        let mut changed = false;
        for template in &mut rules.templates {
            if template.stopped || calendar_date(&template.next_on)? > today {
                continue;
            }
            if next.todos.iter().any(|row| {
                !row.done
                    && row.deleted_at.is_none()
                    && row
                        .recurrence
                        .as_ref()
                        .is_some_and(|occurrence| occurrence.template_id == template.id)
            }) {
                continue;
            }
            let anchor = calendar_date(&template.starts_on)?;
            let Some(scheduled) = template.schedule.latest(anchor, today)? else {
                continue;
            };
            if scheduled < calendar_date(&template.next_on)? {
                continue;
            }
            let date = day_string(scheduled);
            let id = format!("{}@{date}", template.id);
            if next
                .todos
                .iter()
                .chain(next.archive.iter())
                .any(|row| row.id == id)
            {
                return Err("周期实例已存在，请检查周期游标".into());
            }
            next.todos.insert(
                0,
                Todo {
                    id,
                    text: template.fields.text.clone(),
                    notes: template.fields.notes.clone(),
                    urgent: template.fields.urgent,
                    deadline: Some(date.clone()),
                    done: false,
                    completed_at: None,
                    deleted_at: None,
                    resources: template.resources.clone(),
                    recurrence: Some(Occurrence {
                        template_id: template.id.clone(),
                        scheduled_on: date.clone(),
                    }),
                    extra: BTreeMap::new(),
                },
            );
            template.last_emitted = Some(date);
            template.next_on = day_string(template.schedule.on_or_after(anchor, after(today, 1)?)?);
            changed = true;
        }
        if changed {
            rules.bump()?;
            next.revision = next.revision.checked_add(1).ok_or("待办版本超出范围")?;
        }
        rules.validate_links(&next)?;
        Ok(Candidate { rules, todos: next })
    }
    /// Completion advances directly to a future scheduled date, even when the
    /// original occurrence was overdue. Undo preserves the cursor and history.
    pub fn after_todo_change(
        &self,
        before: &todos::Document,
        after_state: &todos::Document,
        today: &str,
    ) -> Result<Document> {
        self.validate_links(before)?;
        let today = calendar_date(today)?;
        let mut next = self.clone();
        let mut changed = false;
        for row in &after_state.todos {
            let Some(occurrence) = &row.recurrence else {
                continue;
            };
            let was_done = before
                .todos
                .iter()
                .find(|old| old.id == row.id)
                .is_some_and(|old| old.done);
            if row.done && !was_done {
                let template = next
                    .templates
                    .iter_mut()
                    .find(|template| template.id == occurrence.template_id)
                    .ok_or("周期规则已不存在")?;
                let date = template
                    .schedule
                    .on_or_after(calendar_date(&template.starts_on)?, after(today, 1)?)?;
                template.next_on = day_string(calendar_date(&template.next_on)?.max(date));
                changed = true;
            }
        }
        if changed {
            next.bump()?;
        }
        next.validate_links(after_state)?;
        Ok(next)
    }
}
