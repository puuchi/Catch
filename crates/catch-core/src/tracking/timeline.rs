use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRef {
    pub activity_id: String,
    pub session_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineGap {
    pub start_time: String,
    pub end_time: String,
}
const EDIT_SNAPSHOT: &str = "catchSessionEditSnapshot";

fn bounds(session: &Session) -> Result<(i64, i64)> {
    Ok((
        timestamp(&session.start_time)?,
        session
            .end_time
            .as_deref()
            .map(timestamp)
            .transpose()?
            .unwrap_or(i64::MAX),
    ))
}
fn rebuild(matter: &mut Matter, now: i64) -> Result<()> {
    let rows = matter.sessions.as_mut().unwrap();
    rows.sort_by_key(|row| timestamp(&row.start_time).unwrap_or(i64::MAX));
    if let Some(first) = rows.first() {
        matter.start_time = first.start_time.clone();
    }
    if rows.iter().any(|row| row.end_time.is_none()) {
        matter.status = Status::Running;
        matter.end_time = None;
        matter.pause_reason = None;
    } else if matter.status == Status::Running {
        matter.status = Status::Paused;
        matter.pause_reason = Some("manual".into());
    }
    if matter.status == Status::Completed {
        matter.end_time = rows
            .iter()
            .filter_map(|row| row.end_time.as_ref())
            .max_by_key(|end| timestamp(end).unwrap_or(i64::MIN))
            .cloned();
    }
    matter.sync_segments();
    matter.updated_at = iso(now)?;
    Ok(())
}
impl Document {
    fn timeline_index(&self, id: &str) -> Result<usize> {
        self.activities
            .iter()
            .position(|row| row.id == id)
            .ok_or_else(|| "事项已不存在".into())
    }
    fn locate_session(&mut self, activity: &str, session: &str) -> Result<(usize, usize)> {
        let i = self.timeline_index(activity)?;
        self.activities[i].ensure_sessions()?;
        let j = self.activities[i]
            .sessions
            .as_ref()
            .unwrap()
            .iter()
            .position(|row| row.id == session)
            .ok_or("本次记录已不存在")?;
        Ok((i, j))
    }
    // Session envelopes prevent crossing another entry even when its effective
    // intervals happen to lie inside a pause. All times are full, unclipped UTC.
    fn unobstructed(&self, from: i64, to: i64, excluded: &[&str]) -> Result<()> {
        for matter in &self.activities {
            // Read legacy identities without migrating unrelated history.
            let mut legacy;
            let rows = match &matter.sessions {
                Some(rows) => rows,
                None => {
                    legacy = matter.clone();
                    legacy.ensure_sessions()?;
                    legacy.sessions.as_ref().unwrap()
                }
            };
            for row in rows {
                if excluded.contains(&row.id.as_str()) {
                    continue;
                }
                let (start, end) = bounds(row)?;
                if (start < to && end > from) || (start == end && start > from && start < to) {
                    return Err("只能合并全局时间轴中相邻的记录，不能跨过其他投入".into());
                }
            }
        }
        Ok(())
    }
    pub(super) fn merge_sessions(
        &mut self,
        first: &SessionRef,
        second: Option<&SessionRef>,
        gap: Option<&TimelineGap>,
        target_id: &str,
        context: Context<'_>,
    ) -> Result<()> {
        if second.is_some() == gap.is_some() {
            return Err("请选择另一条投入或一个未计时段（二选一）".into());
        }
        let (a, ai) = self.locate_session(&first.activity_id, &first.session_id)?;
        let one = self.activities[a].sessions.as_ref().unwrap()[ai].clone();
        if let Some(gap) = gap {
            if target_id != first.activity_id {
                return Err("目标必须是选中投入所属事项".into());
            }
            let from = timestamp(&gap.start_time)?;
            let to = timestamp(&gap.end_time)?;
            let (start, end) = bounds(&one)?;
            if from >= to || to > context.now || !(to == start || from == end) {
                return Err("未计时段必须紧贴本次记录开始或结束，且不能是未来时间".into());
            }
            self.unobstructed(from, to, &[&one.id])?;
            let row = &mut self.activities[a].sessions.as_mut().unwrap()[ai];
            row.segments
                .push(Segment::new(&iso(from)?, Some(&iso(to)?)));
            row.segments.sort_by_key(|s| timestamp(&s.start).unwrap());
            row.start_time = iso(start.min(from))?;
            if end != i64::MAX {
                row.end_time = Some(iso(end.max(to))?);
            }
            return rebuild(&mut self.activities[a], context.now);
        }
        let second = second.unwrap();
        if first.session_id == second.session_id {
            return Err("不能合并同一条投入".into());
        }
        let (b, bi) = self.locate_session(&second.activity_id, &second.session_id)?;
        let two = self.activities[b].sessions.as_ref().unwrap()[bi].clone();
        if target_id != first.activity_id && target_id != second.activity_id {
            return Err("目标必须是选中投入所属事项".into());
        }
        let target = self.timeline_index(target_id)?;
        let source = if target == a { b } else { a };
        let target_todo = self.activities[target].todo_id.as_deref();
        let source_todo = self.activities[source].todo_id.as_deref();
        if source_todo.is_some() && target_todo != source_todo {
            return Err(if target_todo.is_none() {
                "请选择有关联待办的事项为目标，避免丢失关联"
            } else {
                "不同待办关联的投入不能合并"
            }
            .into());
        }
        let (early, late) = if bounds(&one)?.0 <= bounds(&two)?.0 {
            (&one, &two)
        } else {
            (&two, &one)
        };
        let (from, early_end) = bounds(early)?;
        let (late_start, to) = bounds(late)?;
        if early_end > late_start {
            return Err("两条投入的时间范围交错，不能合并".into());
        }
        self.unobstructed(from, to, &[&one.id, &two.id])?;
        let kept = if target == a { &one } else { &two };
        let mut merged = kept.clone();
        merged.start_time = early.start_time.clone();
        merged.end_time = late.end_time.clone();
        merged.progress = [early.progress.as_str(), late.progress.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        merged.segments = early
            .segments
            .iter()
            .chain(&late.segments)
            .cloned()
            .collect();
        if target != source {
            let resources = self.activities[source].resources.clone();
            let dest = &mut self.activities[target].resources;
            for mut resource in resources {
                if dest
                    .iter()
                    .any(|r| r.kind == resource.kind && r.identity() == resource.identity())
                {
                    continue;
                }
                if dest.iter().any(|r| r.id == resource.id) {
                    let mut suffix = 0_u64;
                    loop {
                        let id = format!("merge-resource:{suffix}");
                        if !dest.iter().any(|r| r.id == id) {
                            resource.id = id;
                            break;
                        }
                        suffix += 1;
                    }
                }
                dest.push(resource);
            }
        }
        for i in [a, b] {
            self.activities[i]
                .sessions
                .as_mut()
                .unwrap()
                .retain(|row| row.id != one.id && row.id != two.id);
        }
        self.activities[target]
            .sessions
            .as_mut()
            .unwrap()
            .push(merged);
        rebuild(&mut self.activities[target], context.now)?;
        if source != target {
            if self.activities[source]
                .sessions
                .as_ref()
                .unwrap()
                .is_empty()
            {
                self.activities.remove(source);
            } else {
                rebuild(&mut self.activities[source], context.now)?;
            }
        }
        Ok(())
    }
    pub(super) fn split_session(
        &mut self,
        activity: &str,
        session: &str,
        at: &str,
        context: Context<'_>,
    ) -> Result<()> {
        let (i, j) = self.locate_session(activity, session)?;
        let at = timestamp(at)?;
        let original = self.activities[i].sessions.as_ref().unwrap()[j].clone();
        let (start, end) = bounds(&original)?;
        if at <= start || at >= end.min(context.now) {
            return Err("拆分时间必须严格位于本次投入内部".into());
        }
        let mut before = original.clone();
        let mut after = original.clone();
        before.end_time = Some(iso(at)?);
        after.start_time = iso(at)?;
        after.progress.clear();
        before.segments.clear();
        after.segments.clear();
        let mut durations = [0_i64; 2];
        for segment in &original.segments {
            let from = timestamp(&segment.start)?;
            let to = segment
                .end
                .as_deref()
                .map(timestamp)
                .transpose()?
                .unwrap_or(context.now);
            if from < at && to > from {
                let mut clipped = segment.clone();
                clipped.end = Some(iso(to.min(at))?);
                durations[0] += to.min(at) - from;
                before.segments.push(clipped);
            }
            if to > at && to > from {
                let mut clipped = segment.clone();
                clipped.start = iso(from.max(at))?;
                durations[1] += to - from.max(at);
                after.segments.push(clipped);
            }
        }
        if durations.contains(&0) {
            return Err("拆分后两条投入都必须包含有效时间，不能只有暂停空档".into());
        }
        let matter = &mut self.activities[i];
        let index = matter.next_session_index.unwrap();
        matter.next_session_index = Some(index.checked_add(1).ok_or("投入序号超出范围")?);
        after.id = format!("{}:{index}", matter.id);
        matter.sessions.as_mut().unwrap()[j] = before;
        matter.sessions.as_mut().unwrap().push(after);
        rebuild(matter, context.now)
    }
    pub(super) fn begin_session_edit(
        &mut self,
        activity: &str,
        session: &str,
        draft: Option<&str>,
        context: Context<'_>,
    ) -> Result<()> {
        let (i, j) = self.locate_session(activity, session)?;
        let matter = &mut self.activities[i];
        let rows = matter.sessions.as_mut().unwrap();
        if matter.status != Status::Running || j + 1 != rows.len() || rows[j].end_time.is_some() {
            return Err("只能编辑正在运行的最后一次投入".into());
        }
        if context.new_id.is_empty() {
            return Err("编辑标识无效".into());
        }
        if context.now < timestamp(&rows[j].segments.last().ok_or("缺少有效时间段")?.start)?
        {
            return Err("系统时间早于本次记录".into());
        }
        if let Some(draft) = draft {
            if text_length(draft) > 2000 {
                return Err("本次进展最多 2000 个字符".into());
            }
            rows[j].progress = draft.trim().into();
        }
        matter.pause(context.now, &format!("edit:{}", context.new_id))?;
        let snapshot = serde_json::to_value(&matter.sessions.as_ref().unwrap()[j])
            .map_err(|e| e.to_string())?;
        matter.extra.insert(EDIT_SNAPSHOT.into(), snapshot);
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn finish_session_edit(
        &mut self,
        activity: &str,
        session: &str,
        token: &str,
        cancel: bool,
        start: Option<&str>,
        end: Option<&str>,
        progress: Option<&str>,
        segments: Option<&[Segment]>,
        context: Context<'_>,
    ) -> Result<()> {
        let (i, j) = self.locate_session(activity, session)?;
        let matter = &self.activities[i];
        let original = matter.sessions.as_ref().unwrap()[j].clone();
        if !token.starts_with("edit:")
            || matter.status != Status::Paused
            || matter.pause_reason.as_deref() != Some(token)
            || j + 1 != matter.sessions.as_ref().unwrap().len()
            || matter.extra.get(EDIT_SNAPSHOT)
                != Some(&serde_json::to_value(&original).map_err(|e| e.to_string())?)
        {
            return Err("编辑会话或本次记录已变化，请核对后重试".into());
        }
        if !cancel {
            let command = Command::EditSession {
                activity_id: activity.into(),
                session_id: session.into(),
                start_time: start.unwrap_or(&original.start_time).into(),
                end_time: end
                    .or(original.end_time.as_deref())
                    .ok_or("缺少结束时间")?
                    .into(),
                progress: progress.unwrap_or(&original.progress).into(),
                segments: segments.unwrap_or(&original.segments).to_vec(),
            };
            // Reuse all legacy edit-session validation, but publish only the
            // outer command's revision and single storage write.
            let revision = self.revision;
            *self = self.apply(&command, revision, context)?;
            self.revision = revision;
        }
        let other_running = self.current().is_some();
        let (i, j) = self.locate_session(activity, session)?;
        let matter = &mut self.activities[i];
        let row = &mut matter.sessions.as_mut().unwrap()[j];
        if bounds(row)?.1 > context.now {
            return Err("结束时间不能晚于恢复计时的当前时间".into());
        }
        if !other_running {
            if j + 1 != matter.sessions.as_ref().unwrap().len() {
                return Err("编辑后本次记录不再是最后一次投入".into());
            }
            let row = &mut matter.sessions.as_mut().unwrap()[j];
            row.end_time = None;
            row.segments.push(Segment::new(&iso(context.now)?, None));
            matter.status = Status::Running;
            matter.end_time = None;
        }
        matter.pause_reason = if other_running {
            Some("manual".into())
        } else {
            None
        };
        matter.extra.remove(EDIT_SNAPSHOT);
        rebuild(matter, context.now)
    }
}
