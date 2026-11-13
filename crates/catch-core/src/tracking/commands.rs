use super::*;

/// Supplied once at the serialized application boundary. A renderer may never
/// choose the clock used for live tracking or generate persisted identities.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub now: i64,
    pub new_id: &'a str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    Start {
        name: String,
        #[serde(default)]
        todo_id: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        resources: Vec<crate::resources::Resource>,
    },
    Pause {
        #[serde(default = "manual")]
        reason: String,
        #[serde(default)]
        cutoff: Option<i64>,
    },
    Resume {
        #[serde(default)]
        id: Option<String>,
    },
    End {
        #[serde(default)]
        id: Option<String>,
    },
    Edit {
        #[serde(default)]
        id: Option<String>,
        name: String,
        start_time: String,
        end_time: Option<String>,
        #[serde(default)]
        segments: Vec<Segment>,
        #[serde(default)]
        progress: String,
    },
    Remove {
        id: String,
    },
    AddSession {
        #[serde(default)]
        activity_id: Option<String>,
        #[serde(default)]
        name: String,
        start_time: String,
        end_time: String,
        #[serde(default)]
        progress: String,
    },
    SetProgress {
        activity_id: String,
        session_id: String,
        progress: String,
    },
    NextSession {
        activity_id: String,
        session_id: String,
        progress: String,
    },
    UpdateMatter {
        id: String,
        name: String,
        status: Status,
    },
    EditSession {
        activity_id: String,
        session_id: String,
        start_time: String,
        end_time: String,
        #[serde(default)]
        progress: String,
        segments: Vec<Segment>,
    },
    DeleteSession {
        activity_id: String,
        session_id: String,
    },
    RestoreTrash {
        id: String,
    },
    DeleteTrash {
        id: String,
    },
    SetResources {
        id: String,
        resources: Vec<crate::resources::Resource>,
    },
    Checkpoint,
}
fn manual() -> String {
    "manual".into()
}
fn name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || text_length(value) > 200 {
        return Err("活动名称须为 1–200 个字符".into());
    }
    Ok(value.into())
}
fn progress(value: &str) -> Result<String> {
    if text_length(value) > 2000 {
        return Err("本次进展最多 2000 个字符".into());
    }
    Ok(value.trim().into())
}
fn normalized_segments(segments: &[Segment]) -> Result<Vec<Segment>> {
    segments
        .iter()
        .map(|segment| {
            Ok(Segment::new(
                &iso(timestamp(&segment.start)?)?,
                Some(&iso(timestamp(
                    segment.end.as_deref().ok_or("时间格式无效")?,
                )?)?),
            ))
        })
        .collect()
}
fn fresh(doc: &Document, id: &str, name: String, time: &str, todo_id: Option<String>) -> Matter {
    Matter {
        id: id.into(),
        name,
        color: Some(color::next_color(doc)),
        start_time: time.into(),
        end_time: None,
        status: Status::Running,
        segments: vec![],
        sessions: Some(vec![]),
        next_session_index: Some(0),
        todo_id,
        resources: vec![],
        created_at: time.into(),
        updated_at: time.into(),
        pause_reason: None,
        extra: Extensions::new(),
    }
}
impl Document {
    fn index(&self, id: &str) -> Result<usize> {
        self.activities
            .iter()
            .position(|matter| matter.id == id)
            .ok_or_else(|| "事项已不存在".into())
    }
    fn pause_current(&mut self, now: i64) -> Result<()> {
        if let Some(current) = self
            .activities
            .iter_mut()
            .find(|matter| matter.status == Status::Running)
        {
            current.pause(now, "switch")?;
        }
        Ok(())
    }
    /// Returns a fully validated candidate, leaving `self` untouched on both
    /// success and failure. Storage commit is intentionally a separate step.
    pub fn apply(
        &self,
        command: &Command,
        expected_revision: u64,
        context: Context<'_>,
    ) -> Result<Self> {
        if expected_revision != self.revision {
            return Err("记录已变化，请核对后重试；草稿已保留".into());
        }
        self.validate()?;
        let mut next = self.clone();
        let now = context.now;
        let time = iso(now)?;
        match command {
            Command::Start {
                name: input,
                todo_id,
                resources,
            } => {
                let name = name(input)?;
                next.pause_current(now)?;
                let mut matter = fresh(&next, context.new_id, name, &time, todo_id.clone());
                matter.resources = resources.clone();
                matter.append(&time)?;
                next.no_overlap(&matter)?;
                next.activities.push(matter);
            }
            Command::Pause { reason, cutoff } => {
                let Some(index) = next
                    .activities
                    .iter()
                    .position(|matter| matter.status == Status::Running)
                else {
                    return Ok(next);
                };
                next.activities[index].pause(cutoff.unwrap_or(now), reason)?;
                next.activities[index].updated_at = time.clone();
            }
            Command::Resume { id } => {
                let paused: Vec<_> = next
                    .activities
                    .iter()
                    .filter(|matter| matter.status == Status::Paused)
                    .collect();
                if id.is_none() && paused.len() > 1 {
                    return Err("请选择要继续的后台事项".into());
                }
                let id = id
                    .as_deref()
                    .or_else(|| paused.first().map(|matter| matter.id.as_str()))
                    .ok_or("当前没有可继续的活动")?
                    .to_owned();
                if next.current().is_some_and(|matter| matter.id == id) {
                    return Ok(next);
                }
                let index = next.index(&id)?;
                let target = &next.activities[index];
                if target.status != Status::Paused {
                    return Err("当前没有可继续的活动".into());
                }
                if now < timestamp(&target.start_time)?
                    || target
                        .segments
                        .last()
                        .and_then(|segment| segment.end.as_deref())
                        .map(timestamp)
                        .transpose()?
                        .is_some_and(|end| now < end)
                {
                    return Err("系统时间早于上次记录，请检查系统时钟".into());
                }
                next.pause_current(now)?;
                let target = &mut next.activities[index];
                target.append(&time)?;
                target.status = Status::Running;
                target.pause_reason = None;
                target.updated_at = time.clone();
                next.no_overlap(&next.activities[index])?;
            }
            Command::End { id } => {
                let index = match id {
                    Some(id) => next.index(id)?,
                    None => match next
                        .activities
                        .iter()
                        .position(|matter| matter.status == Status::Running)
                    {
                        Some(index) => index,
                        None => return Ok(next),
                    },
                };
                if next.activities[index].status == Status::Completed {
                    return Ok(next);
                }
                next.activities[index].finish(now)?;
            }
            Command::Edit {
                id,
                name: input,
                start_time,
                end_time,
                segments,
                progress: input_progress,
            } => {
                let index = id.as_deref().map(|id| next.index(id)).transpose()?;
                let existing = index.map(|index| next.activities[index].clone());
                if existing
                    .as_ref()
                    .is_some_and(|matter| matter.status == Status::Running)
                {
                    return Err("请先暂停活动，再修改时间记录".into());
                }
                let start = iso(timestamp(start_time)?)?;
                let end = end_time
                    .as_deref()
                    .map(timestamp)
                    .transpose()?
                    .map(iso)
                    .transpose()?;
                if existing.is_none() && end.is_none() {
                    return Err("补录活动必须填写结束时间".into());
                }
                let mut matter = existing
                    .clone()
                    .unwrap_or_else(|| fresh(&next, context.new_id, String::new(), &time, None));
                matter.name = input.trim().into();
                matter.start_time = start;
                matter.end_time = end;
                matter.status = if matter.end_time.is_some() {
                    Status::Completed
                } else {
                    Status::Paused
                };
                matter.segments = normalized_segments(segments)?;
                matter.updated_at = time.clone();
                matter.pause_reason = if matter.end_time.is_some() {
                    None
                } else {
                    Some("manual".into())
                };
                matter.validate()?;
                if timestamp(&matter.start_time)? > now
                    || matter
                        .end_time
                        .as_deref()
                        .map(timestamp)
                        .transpose()?
                        .is_some_and(|end| end > now)
                    || matter.segments.iter().any(|segment| {
                        timestamp(segment.end.as_deref().unwrap()).is_ok_and(|end| end > now)
                    })
                {
                    return Err("不能补录未来的活动时间".into());
                }
                next.no_overlap(&matter)?;
                if existing
                    .as_ref()
                    .is_none_or(|old| old.segments != matter.segments)
                {
                    let mut old = existing.clone();
                    if let Some(old) = old.as_mut() {
                        old.ensure_sessions()?;
                    }
                    if old
                        .as_ref()
                        .is_some_and(|old| old.sessions.as_ref().unwrap().len() > 1)
                    {
                        return Err("请在事项详情中逐次编辑记录".into());
                    }
                    let previous = old
                        .as_ref()
                        .and_then(|old| old.sessions.as_ref().unwrap().first());
                    matter.sessions = Some(vec![Session {
                        id: previous
                            .map(|session| session.id.clone())
                            .unwrap_or_else(|| format!("{}:0", matter.id)),
                        start_time: matter.start_time.clone(),
                        end_time: Some(
                            matter
                                .end_time
                                .clone()
                                .or_else(|| {
                                    matter
                                        .segments
                                        .last()
                                        .and_then(|segment| segment.end.clone())
                                })
                                .unwrap_or_else(|| matter.start_time.clone()),
                        ),
                        progress: previous
                            .map(|session| session.progress.clone())
                            .unwrap_or_else(|| input_progress.clone()),
                        segments: matter.segments.clone(),
                        extra: Extensions::new(),
                    }]);
                    matter.next_session_index = Some(
                        existing
                            .as_ref()
                            .and_then(|matter| matter.next_session_index)
                            .filter(|value| *value > 0)
                            .unwrap_or(1),
                    );
                }
                if let Some(index) = index {
                    next.activities[index] = matter;
                } else {
                    next.activities.push(matter);
                }
            }
            Command::Remove { id } => {
                let index = next.index(id)?;
                let mut matter = next.activities.remove(index);
                if matter.status == Status::Running {
                    matter.pause(now, "trash")?;
                }
                next.trash.push(TrashEntry {
                    id: context.new_id.into(),
                    kind: TrashKind::Matter,
                    deleted_at: time.clone(),
                    activity: matter,
                    session: None,
                    extra: Extensions::new(),
                });
            }
            Command::AddSession {
                activity_id,
                name: input,
                start_time,
                end_time,
                progress: input_progress,
            } => {
                let start = iso(timestamp(start_time)?)?;
                let end = iso(timestamp(end_time)?)?;
                if timestamp(&end)? > now {
                    return Err("不能补录未来的活动时间".into());
                }
                let progress = progress(input_progress)?;
                let index = activity_id
                    .as_deref()
                    .map(|id| next.index(id))
                    .transpose()?;
                let mut matter = if let Some(index) = index {
                    next.activities[index].clone()
                } else {
                    let mut matter = fresh(&next, context.new_id, input.trim().into(), &time, None);
                    matter.start_time = start.clone();
                    matter.end_time = Some(end.clone());
                    matter.status = Status::Completed;
                    matter
                };
                matter.ensure_sessions()?;
                let session_index = matter.next_session_index.unwrap();
                matter.next_session_index =
                    Some(session_index.checked_add(1).ok_or("投入序号超出范围")?);
                let session = Session {
                    id: format!("{}:{session_index}", matter.id),
                    start_time: start.clone(),
                    end_time: Some(end.clone()),
                    progress,
                    segments: vec![Segment::new(&start, Some(&end))],
                    extra: Extensions::new(),
                };
                let mut candidate = matter.clone();
                candidate.start_time = start.clone();
                candidate.end_time = Some(end.clone());
                candidate.status = Status::Completed;
                candidate.segments = session.segments.clone();
                candidate.validate()?;
                candidate.id = session.id.clone();
                next.no_overlap(&candidate)?;
                matter.sessions.as_mut().unwrap().push(session);
                matter
                    .sessions
                    .as_mut()
                    .unwrap()
                    .sort_by_key(|session| timestamp(&session.start_time).unwrap());
                matter.sync_segments();
                matter.start_time = iso(timestamp(&matter.start_time)?.min(timestamp(&start)?))?;
                if matter.status == Status::Completed {
                    matter.end_time = Some(iso(
                        timestamp(matter.end_time.as_deref().unwrap())?.max(timestamp(&end)?)
                    )?);
                }
                matter.updated_at = time.clone();
                if let Some(index) = index {
                    next.activities[index] = matter;
                } else {
                    next.activities.push(matter);
                }
            }
            Command::SetProgress {
                activity_id,
                session_id,
                progress: input_progress,
            } => {
                let progress = progress(input_progress)?;
                let index = next.index(activity_id)?;
                let target = &mut next.activities[index];
                target.ensure_sessions()?;
                let session = target
                    .sessions
                    .as_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|session| session.id == *session_id)
                    .ok_or("本次记录已不存在")?;
                session.progress = progress;
                target.updated_at = time.clone();
            }
            Command::NextSession {
                activity_id,
                session_id,
                progress: input_progress,
            } => {
                let progress = progress(input_progress)?;
                let target = next
                    .activities
                    .iter_mut()
                    .find(|matter| matter.status == Status::Running)
                    .ok_or("当前投入已变化，请核对后重试")?;
                target.ensure_sessions()?;
                let session = target
                    .sessions
                    .as_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|session| session.end_time.is_none())
                    .ok_or("当前投入已变化，请核对后重试")?;
                if target.id != *activity_id || session.id != *session_id {
                    return Err("当前投入已变化，请核对后重试".into());
                }
                if now < timestamp(&session.start_time)? {
                    return Err("系统时间早于本次开始时间，请检查系统时钟".into());
                }
                session.progress = progress;
                target.close(now)?;
                target.append(&time)?;
                target.updated_at = time.clone();
                next.no_overlap(next.current().unwrap())?;
            }
            Command::UpdateMatter {
                id,
                name: input,
                status,
            } => {
                let name = name(input)?;
                let index = next.index(id)?;
                next.activities[index].name = name;
                if *status == Status::Completed
                    && next.activities[index].status != Status::Completed
                {
                    next.activities[index].finish(now)?;
                }
                if *status == Status::Paused && next.activities[index].status != Status::Paused {
                    next.activities[index].pause(now, "manual")?;
                    next.activities[index].end_time = None;
                }
                if *status == Status::Running && next.activities[index].status != Status::Running {
                    if next.activities[index]
                        .segments
                        .last()
                        .and_then(|segment| segment.end.as_deref())
                        .map(timestamp)
                        .transpose()?
                        .is_some_and(|end| now < end)
                    {
                        return Err("系统时间早于上次记录".into());
                    }
                    next.pause_current(now)?;
                    let target = &mut next.activities[index];
                    target.append(&time)?;
                    target.status = Status::Running;
                    target.end_time = None;
                    target.pause_reason = None;
                }
                next.activities[index].updated_at = time.clone();
                next.no_overlap(&next.activities[index])?;
            }
            Command::EditSession {
                activity_id,
                session_id,
                start_time,
                end_time,
                progress,
                segments,
            } => {
                let index = next.index(activity_id)?;
                let target = &mut next.activities[index];
                target.ensure_sessions()?;
                let session_index = target
                    .sessions
                    .as_ref()
                    .unwrap()
                    .iter()
                    .position(|session| session.id == *session_id)
                    .ok_or("本次记录已不存在")?;
                if target.sessions.as_ref().unwrap()[session_index]
                    .end_time
                    .is_none()
                {
                    return Err("请先暂停本次记录，再修改时间".into());
                }
                let edited = Session {
                    id: session_id.clone(),
                    start_time: iso(timestamp(start_time)?)?,
                    end_time: Some(iso(timestamp(end_time)?)?),
                    progress: progress.trim().into(),
                    segments: normalized_segments(segments)?,
                    extra: target.sessions.as_ref().unwrap()[session_index]
                        .extra
                        .clone(),
                };
                let mut candidate = target.clone();
                candidate.id = edited.id.clone();
                candidate.start_time = edited.start_time.clone();
                candidate.end_time = edited.end_time.clone();
                candidate.status = Status::Completed;
                candidate.segments = edited.segments.clone();
                candidate.validate()?;
                if timestamp(edited.end_time.as_deref().unwrap())? > now
                    || edited.segments.iter().any(|segment| {
                        timestamp(segment.end.as_deref().unwrap()).is_ok_and(|end| end > now)
                    })
                {
                    return Err("不能补录未来的活动时间".into());
                }
                target.sessions.as_mut().unwrap()[session_index] = edited;
                target
                    .sessions
                    .as_mut()
                    .unwrap()
                    .sort_by_key(|session| timestamp(&session.start_time).unwrap());
                target.sync_segments();
                target.start_time = target.sessions.as_ref().unwrap()[0].start_time.clone();
                if let Some(end) = target.end_time.as_deref() {
                    let end = target
                        .sessions
                        .as_ref()
                        .unwrap()
                        .iter()
                        .filter_map(|session| session.end_time.as_deref())
                        .try_fold(timestamp(end)?, |maximum, end| {
                            Ok::<_, String>(maximum.max(timestamp(end)?))
                        })?;
                    target.end_time = Some(iso(end)?);
                }
                target.updated_at = time.clone();
                next.no_overlap(&next.activities[index])?;
            }
            Command::DeleteSession {
                activity_id,
                session_id,
            } => {
                let index = next.index(activity_id)?;
                let target = &mut next.activities[index];
                target.ensure_sessions()?;
                let session_index = target
                    .sessions
                    .as_ref()
                    .unwrap()
                    .iter()
                    .position(|session| session.id == *session_id)
                    .ok_or("本次记录已不存在")?;
                if target.sessions.as_ref().unwrap()[session_index]
                    .end_time
                    .is_none()
                {
                    target.pause(now, "manual")?;
                }
                let mut parent = target.clone();
                parent.sessions = Some(vec![]);
                parent.segments = vec![];
                if parent.status == Status::Running {
                    parent.status = Status::Paused;
                    parent.pause_reason = Some("trash".into());
                }
                let session = target.sessions.as_mut().unwrap().remove(session_index);
                target.sync_segments();
                target.updated_at = time.clone();
                next.trash.push(TrashEntry {
                    id: context.new_id.into(),
                    kind: TrashKind::Session,
                    deleted_at: time.clone(),
                    activity: parent,
                    session: Some(session),
                    extra: Extensions::new(),
                });
            }
            Command::RestoreTrash { id } => {
                let trash_index = next
                    .trash
                    .iter()
                    .position(|entry| entry.id == *id)
                    .ok_or("回收站记录已不存在")?;
                let entry = &next.trash[trash_index];
                let mut incoming = entry.activity.clone();
                if entry.kind == TrashKind::Session {
                    incoming.sessions =
                        Some(vec![entry.session.clone().ok_or("回收站缺少本次记录")?]);
                }
                incoming.ensure_sessions()?;
                let index = if let Some(index) = next
                    .activities
                    .iter()
                    .position(|matter| matter.id == incoming.id)
                {
                    let target = &mut next.activities[index];
                    target.ensure_sessions()?;
                    if incoming.sessions.as_ref().unwrap().iter().any(|session| {
                        target
                            .sessions
                            .as_ref()
                            .unwrap()
                            .iter()
                            .any(|existing| existing.id == session.id)
                    }) {
                        return Err("同一记录已存在，无法重复恢复".into());
                    }
                    target
                        .sessions
                        .as_mut()
                        .unwrap()
                        .extend(incoming.sessions.unwrap());
                    target.next_session_index = Some(
                        target
                            .next_session_index
                            .unwrap()
                            .max(incoming.next_session_index.unwrap()),
                    );
                    index
                } else {
                    next.activities.push(incoming);
                    next.activities.len() - 1
                };
                let target = &mut next.activities[index];
                target
                    .sessions
                    .as_mut()
                    .unwrap()
                    .sort_by_key(|session| timestamp(&session.start_time).unwrap());
                target.sync_segments();
                let start = target.sessions.as_ref().unwrap().iter().try_fold(
                    timestamp(&target.start_time)?,
                    |minimum, session| {
                        Ok::<_, String>(minimum.min(timestamp(&session.start_time)?))
                    },
                )?;
                target.start_time = iso(start)?;
                if target.status == Status::Completed {
                    let end = target
                        .sessions
                        .as_ref()
                        .unwrap()
                        .iter()
                        .filter_map(|session| session.end_time.as_deref())
                        .try_fold(
                            timestamp(target.end_time.as_deref().unwrap())?,
                            |maximum, end| Ok::<_, String>(maximum.max(timestamp(end)?)),
                        )?;
                    target.end_time = Some(iso(end)?);
                }
                target.updated_at = time.clone();
                next.no_overlap(&next.activities[index])?;
                next.trash.remove(trash_index);
            }
            Command::DeleteTrash { id } => {
                let index = next
                    .trash
                    .iter()
                    .position(|entry| entry.id == *id)
                    .ok_or("回收站记录已不存在")?;
                next.trash.remove(index);
            }
            Command::SetResources { id, resources } => {
                crate::resources::validate_list(resources)?;
                let index = next.index(id)?;
                next.activities[index].resources = resources.clone();
                next.activities[index].updated_at = time.clone();
            }
            Command::Checkpoint => {}
        }
        next.revision = self.revision.checked_add(1).ok_or("记录版本超出范围")?;
        next.checkpoint_at = Some(time);
        next.validate()?;
        Ok(next)
    }
}
