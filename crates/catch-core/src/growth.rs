use crate::{
    todos::calendar_date,
    tracking::{iso, timestamp, Document as Tracking},
    Result,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Plant {
    Tulip,
    Sunflower,
    Corn,
    Strawberry,
    #[default]
    Sprout,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub plant: Plant,
    pub chosen_at: String,
    pub celebrated_at: Option<String>,
    pub goal_on_celebration: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub version: u32,
    pub revision: u64,
    pub goal_minutes: Option<u32>,
    pub preferred_plant: Plant,
    pub days: BTreeMap<String, Day>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            goal_minutes: Some(60),
            preferred_plant: Plant::Sprout,
            days: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "kebab-case")]
pub enum Command {
    Choose { plant: Plant },
    SetGoal { minutes: Option<u32> },
}
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub now: i64,
    pub local_day: &'a str,
}
/// Resolved by the trusted host using its local timezone. DST days may be 23
/// or 25 hours; historical totals use these boundaries rather than 86400000.
pub struct DayBounds<'a> {
    pub day: &'a str,
    pub start: i64,
    pub end: i64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub day: String,
    pub plant: Plant,
    pub selected: bool,
    pub invested_ms: i64,
    pub goal_minutes: Option<u32>,
    pub stage: u8,
    pub reached: bool,
    pub celebrated: bool,
}
fn goal(minutes: Option<u32>) -> Result<()> {
    if minutes.is_some_and(|minutes| !(1..=1440).contains(&minutes)) {
        return Err("每日目标须为 1–1440 分钟，或关闭目标".into());
    }
    Ok(())
}
pub fn invested(tracking: &Tracking, bounds: &DayBounds<'_>, now: i64) -> Result<i64> {
    calendar_date(bounds.day)?;
    if bounds.end <= bounds.start {
        return Err("本地日期范围无效".into());
    }
    tracking.validate()?;
    tracking.activities.iter().try_fold(0_i64, |total, matter| {
        total
            .checked_add(matter.duration(now, Some((bounds.start, bounds.end)))?)
            .ok_or_else(|| "累计时长超出范围".into())
    })
}
impl Document {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err("无法识别成长数据版本，请保留文件并使用兼容版本".into());
        }
        goal(self.goal_minutes)?;
        for (date, day) in &self.days {
            calendar_date(date)?;
            timestamp(&day.chosen_at)?;
            if let Some(time) = &day.celebrated_at {
                timestamp(time)?;
            }
            if day.celebrated_at.is_some() != day.goal_on_celebration.is_some() {
                return Err("成长达成记录不完整".into());
            }
            goal(day.goal_on_celebration)?;
        }
        Ok(())
    }
    fn bump(&mut self) -> Result<()> {
        self.revision = self.revision.checked_add(1).ok_or("成长版本超出范围")?;
        Ok(())
    }
    pub fn apply(&self, command: &Command, revision: u64, context: Context<'_>) -> Result<Self> {
        self.validate()?;
        calendar_date(context.local_day)?;
        if revision != self.revision {
            return Err("今日植物或目标已变化，请核对后重试".into());
        }
        let mut next = self.clone();
        match command {
            Command::Choose { plant } => {
                if let Some(day) = next.days.get(context.local_day) {
                    if day.plant == *plant {
                        return Ok(next);
                    }
                    return Err("今天已选定一种植物，明天再换一种吧".into());
                }
                next.days.insert(
                    context.local_day.into(),
                    Day {
                        plant: *plant,
                        chosen_at: iso(context.now)?,
                        celebrated_at: None,
                        goal_on_celebration: None,
                    },
                );
                next.preferred_plant = *plant;
            }
            Command::SetGoal { minutes } => {
                goal(*minutes)?;
                if next.goal_minutes == *minutes {
                    return Ok(next);
                }
                next.goal_minutes = *minutes;
            }
        }
        next.bump()?;
        next.validate()?;
        Ok(next)
    }
    /// Only the application service calls this with derived effective duration.
    /// Previewing a plant never invokes this method or creates a day record.
    pub fn observe(&self, context: Context<'_>, invested_ms: i64) -> Result<Self> {
        self.validate()?;
        calendar_date(context.local_day)?;
        if invested_ms < 0 {
            return Err("投入时长无效".into());
        }
        let mut next = self.clone();
        if let (Some(minutes), Some(day)) =
            (next.goal_minutes, next.days.get_mut(context.local_day))
        {
            if day.celebrated_at.is_none() && invested_ms >= i64::from(minutes) * 60_000 {
                day.celebrated_at = Some(iso(context.now)?);
                day.goal_on_celebration = Some(minutes);
                next.bump()?;
            }
        }
        Ok(next)
    }
    pub fn view(&self, date: &str, invested_ms: i64) -> Result<View> {
        self.validate()?;
        calendar_date(date)?;
        if invested_ms < 0 {
            return Err("投入时长无效".into());
        }
        let day = self.days.get(date);
        // Disabling a target keeps the growth appearance, using the default
        // hour as visual scale, but produces no goal achievement or reward.
        let denominator = i64::from(self.goal_minutes.unwrap_or(60)) * 60_000;
        let percent = i128::from(invested_ms) * 100 / i128::from(denominator);
        let stage = if percent >= 100 {
            4
        } else if percent >= 60 {
            3
        } else if percent >= 30 {
            2
        } else if percent >= 10 {
            1
        } else {
            0
        };
        Ok(View {
            day: date.into(),
            plant: day.map(|day| day.plant).unwrap_or(self.preferred_plant),
            selected: day.is_some(),
            invested_ms,
            goal_minutes: self.goal_minutes,
            stage,
            reached: self.goal_minutes.is_some() && invested_ms >= denominator,
            celebrated: day.is_some_and(|day| day.celebrated_at.is_some()),
        })
    }
}
