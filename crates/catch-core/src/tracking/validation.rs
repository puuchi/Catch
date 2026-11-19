use super::*;
use std::collections::HashSet;

pub(super) fn validate_intervals(
    start: &str,
    end: Option<&str>,
    status: &Status,
    segments: &[Segment],
) -> Result<()> {
    let start = timestamp(start)?;
    let end = end.map(timestamp).transpose()?.unwrap_or(i64::MAX);
    if end < start || (*status == Status::Completed) != (end != i64::MAX) {
        return Err("结束时间必须晚于或等于开始时间".into());
    }
    let mut previous = start;
    let mut open = 0;
    for (index, segment) in segments.iter().enumerate() {
        let from = timestamp(&segment.start)?;
        let to = segment
            .end
            .as_deref()
            .map(timestamp)
            .transpose()?
            .unwrap_or(i64::MAX);
        if from < previous || to < from || to > end {
            return Err("有效时间段必须按顺序排列，且位于开始和结束时间之间".into());
        }
        if segment.end.is_none() {
            open += 1;
            if *status != Status::Running || index + 1 != segments.len() {
                return Err("只有进行中的最后一段可以没有结束时间".into());
            }
        }
        previous = to;
    }
    if (*status == Status::Running && open != 1) || open > 1 {
        return Err("进行中活动必须有一个开放时间段".into());
    }
    Ok(())
}
impl Matter {
    pub fn validate(&self) -> Result<()> {
        crate::resources::validate_list(&self.resources)?;
        if self.id.is_empty() {
            return Err("记录 ID 无效".into());
        }
        if self.name.trim().is_empty() || text_length(&self.name) > 200 {
            return Err("活动名称须为 1–200 个字符".into());
        }
        if self
            .color
            .as_deref()
            .is_some_and(|value| !color::valid(value))
        {
            return Err("事项颜色无效".into());
        }
        validate_intervals(
            &self.start_time,
            self.end_time.as_deref(),
            &self.status,
            &self.segments,
        )
    }
}
fn validate_matters(matters: &[Matter], version: u32) -> Result<()> {
    let mut ids = HashSet::new();
    let mut session_ids = HashSet::new();
    let mut active = 0;
    let mut intervals = vec![];
    for matter in matters {
        matter.validate()?;
        if !ids.insert(&matter.id) {
            return Err("存在重复记录 ID".into());
        }
        if matter.status == Status::Running || (version == 1 && matter.status == Status::Paused) {
            active += 1;
        }
        if let Some(sessions) = &matter.sessions {
            for session in sessions {
                if session.id.is_empty() || !session_ids.insert(&session.id) {
                    return Err("本次记录 ID 无效或重复".into());
                }
                if text_length(&session.progress) > 2000 {
                    return Err("本次进展最多 2000 个字符".into());
                }
                let status = if session.end_time.is_none() {
                    Status::Running
                } else {
                    Status::Completed
                };
                validate_intervals(
                    &session.start_time,
                    session.end_time.as_deref(),
                    &status,
                    &session.segments,
                )?;
                if session.end_time.is_none() && matter.status != Status::Running {
                    return Err("暂停事项不能包含正在计时的记录".into());
                }
            }
            let mut flat: Vec<_> = sessions
                .iter()
                .flat_map(|session| session.segments.clone())
                .collect();
            flat.sort_by_key(|segment| timestamp(&segment.start).unwrap_or(i64::MAX));
            if flat != matter.segments {
                return Err("本次记录与有效时间段不一致".into());
            }
        }
        for segment in &matter.segments {
            let start = timestamp(&segment.start)?;
            let end = segment
                .end
                .as_deref()
                .map(timestamp)
                .transpose()?
                .unwrap_or(i64::MAX);
            if end > start {
                intervals.push((start, end));
            }
        }
    }
    if active > 1 {
        return Err("存在多项前台计时活动".into());
    }
    intervals.sort_by_key(|interval| interval.0);
    if intervals.windows(2).any(|pair| pair[1].0 < pair[0].1) {
        return Err("不同事项的有效时间段重叠".into());
    }
    Ok(())
}
impl Document {
    pub fn validate(&self) -> Result<()> {
        if ![1, 2].contains(&self.version) {
            return Err("无法识别活动数据版本，请保留文件并使用兼容版本".into());
        }
        if let Some(value) = &self.checkpoint_at {
            timestamp(value)?;
        }
        if self.session_version.is_some_and(|version| version != 1) {
            return Err("无法识别本次记录的数据版本".into());
        }
        validate_matters(&self.activities, self.version)?;
        let mut ids = HashSet::new();
        for entry in &self.trash {
            if entry.id.is_empty() || !ids.insert(&entry.id) {
                return Err("回收站记录 ID 无效或重复".into());
            }
            timestamp(&entry.deleted_at)?;
            entry.activity.validate()?;
            if entry.activity.status == Status::Running {
                return Err("回收站事项必须停止计时".into());
            }
            let mut matter = entry.activity.clone();
            if entry.kind == TrashKind::Session {
                let session = entry.session.as_ref().ok_or("回收站缺少本次记录")?;
                matter.start_time = session.start_time.clone();
                matter.end_time = session.end_time.clone();
                matter.status = Status::Completed;
                matter.sessions = Some(vec![session.clone()]);
                matter.segments = session.segments.clone();
            }
            validate_matters(&[matter], 2)?;
        }
        Ok(())
    }
    pub(super) fn no_overlap(&self, candidate: &Matter) -> Result<()> {
        for other in self
            .activities
            .iter()
            .filter(|other| other.id != candidate.id)
        {
            for a in &candidate.segments {
                for b in &other.segments {
                    let start = timestamp(&a.start)?.max(timestamp(&b.start)?);
                    let end = a
                        .end
                        .as_deref()
                        .map(timestamp)
                        .transpose()?
                        .unwrap_or(i64::MAX)
                        .min(
                            b.end
                                .as_deref()
                                .map(timestamp)
                                .transpose()?
                                .unwrap_or(i64::MAX),
                        );
                    if start < end {
                        return Err(format!(
                            "时间冲突：与「{}」的有效时间段重叠，请调整时间",
                            other.name
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
