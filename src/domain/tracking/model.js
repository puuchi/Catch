const { ensureSessions, appendSession, syncSegments, sessionsOf } = require('./sessions')
const { iso, copy, timestamp, validateActivity, validateDocument, nextMatterColor, assertNoOverlap } = require('./validation')
// Original user-authored activity rules; I/O lives outside this class.
class TrackingModel {
  constructor(data, { now, newId }) { this.data = data; this.now = now; this.newId = newId }
  current(data = this.data) { return data.activities.find(item => item.status === 'running') }
  paused(data = this.data) { return data.activities.filter(item => item.status === 'paused') }
  get(id, data = this.data) { return data.activities.find(item => item.id === id) }
  snapshot() { return copy(this.data) }
  commit(change) {
    const draft = copy(this.data)
    const time = this.now()
    change(draft, time)
    draft.revision = (this.data.revision || 0) + 1
    draft.checkpointAt = iso(time)
    validateDocument(draft)
    this.data = draft
    return this.snapshot()
  }
  closeSegment(activity, time) {
    ensureSessions(activity)
    for (const session of activity.sessions) {
      const segment = session.segments[session.segments.length - 1]
      if (segment && segment.end === null) {
        segment.end = iso(Math.max(timestamp(segment.start), time))
        session.endTime = segment.end
      }
    }
    syncSegments(activity)
  }

  finish(activity, time) {
    this.closeSegment(activity, time)
    const last = activity.segments[activity.segments.length - 1]
    activity.endTime = iso(Math.max(time, timestamp(activity.startTime), last ? timestamp(last.end) : 0))
    activity.status = 'completed'
    activity.pauseReason = null
    activity.updatedAt = activity.endTime
  }

  pauseRecord(activity, time, reason) {
    this.closeSegment(activity, time)
    activity.status = 'paused'
    activity.pauseReason = reason
    activity.updatedAt = iso(time)
  }

  start(name, todoId = null) {
    name = typeof name === 'string' ? name.trim() : ''
    if (!name || name.length > 200) throw new Error('活动名称须为 1–200 个字符')
    return this.commit((draft, time) => {
      const current = this.current(draft)
      if (current) this.pauseRecord(current, time, 'switch')
      const activity = { id: this.newId(), name, color:nextMatterColor(draft), startTime: iso(time), endTime: null, status: 'running',
        segments: [], sessions: [], nextSessionIndex:0, todoId: typeof todoId === 'string' ? todoId : null,
        createdAt: iso(time), updatedAt: iso(time), pauseReason: null }
      appendSession(activity,time)
      assertNoOverlap(activity, draft.activities)
      draft.activities.push(activity)
    })
  }

  pause(reason = 'manual', cutoff) {
    if (this.current()?.status !== 'running') return this.snapshot()
    return this.commit((draft, time) => {
      const current = this.current(draft)
      this.pauseRecord(current, cutoff === undefined ? time : cutoff, reason)
      current.updatedAt = iso(time)
    })
  }

  resume(id) {
    const paused = this.paused()
    if (!id && paused.length > 1) throw new Error('请选择要继续的后台事项')
    const targetId = id || paused[0]?.id
    if (targetId && this.current()?.id === targetId) return this.snapshot()
    return this.commit((draft, time) => {
      const current = this.get(targetId, draft)
      if (!current || current.status !== 'paused') throw new Error('当前没有可继续的活动')
      const last = current.segments[current.segments.length - 1]
      if (time < timestamp(current.startTime) || (last && time < timestamp(last.end))) throw new Error('系统时间早于上次记录，请检查系统时钟')
      const foreground = this.current(draft)
      if (foreground) this.pauseRecord(foreground, time, 'switch')
      appendSession(current,time)
      current.status = 'running'
      current.pauseReason = null
      current.updatedAt = iso(time)
      assertNoOverlap(current, draft.activities)
    })
  }

  end(id) {
    const activity = id ? this.get(id) : this.current()
    if (!activity && !id) return this.snapshot()
    if (!activity) throw new Error('事项已不存在')
    if (activity.status === 'completed') return this.snapshot()
    return this.commit((draft, time) => this.finish(this.get(activity.id, draft), time))
  }

  edit(input) {
    return this.commit((draft, time) => {
      const existing = input.id ? draft.activities.find(activity => activity.id === input.id) : null
      if (input.id && !existing) throw new Error('记录已不存在，请刷新时间轴')
      if (existing?.status === 'running') throw new Error('请先暂停活动，再修改时间记录')
      const startTime = iso(timestamp(input.startTime))
      const endTime = input.endTime ? iso(timestamp(input.endTime)) : null
      if (!existing && !endTime) throw new Error('补录活动必须填写结束时间')
      const activity = { ...existing, id: existing?.id || this.newId(), name: String(input.name || '').trim(), color:existing?.color || nextMatterColor(draft),
        startTime, endTime, status: endTime ? 'completed' : 'paused',
        segments: Array.isArray(input.segments) ? input.segments.map(segment => ({start: iso(timestamp(segment.start)), end: iso(timestamp(segment.end))})) : [],
        todoId: existing?.todoId || null, createdAt: existing?.createdAt || iso(time), updatedAt: iso(time), pauseReason: endTime ? null : 'manual' }
      validateActivity(activity)
      if (timestamp(startTime) > time || (endTime && timestamp(endTime) > time) || activity.segments.some(segment => timestamp(segment.end) > time)) throw new Error('不能补录未来的活动时间')
      assertNoOverlap(activity, draft.activities)
      const sameIntervals = existing && JSON.stringify(existing.segments) === JSON.stringify(activity.segments)
      if (!sameIntervals) {
        if (existing && sessionsOf(existing).length > 1) throw new Error('请在事项详情中逐次编辑记录')
        const previous = existing && sessionsOf(existing)[0]
        activity.sessions = [{id:previous?.id || `${activity.id}:0`, startTime:activity.startTime,
          endTime:activity.endTime || activity.segments.at(-1)?.end || activity.startTime,
          progress:previous?.progress || String(input.progress || ''), segments:copy(activity.segments)}]
        activity.nextSessionIndex = existing?.nextSessionIndex || 1
      }
      if (existing) draft.activities[draft.activities.indexOf(existing)] = activity
      else draft.activities.push(activity)
    })
  }

  remove(id) {
    return this.commit((draft,time) => {
      const activity = this.get(id,draft)
      if (!activity) throw new Error('记录已不存在')
      if (activity.status === 'running') this.pauseRecord(activity,time,'trash')
      draft.trash ||= []
      draft.trash.push({id:this.newId(),kind:'matter',deletedAt:iso(time),activity:copy(activity)})
      draft.activities = draft.activities.filter(activity => activity.id !== id)
    })
  }

  addSession(input) {
    return this.commit((draft,time) => {
      const startTime = iso(timestamp(input.startTime))
      const endTime = iso(timestamp(input.endTime))
      if (timestamp(endTime) > time) throw new Error('不能补录未来的活动时间')
      const progress = input.progress === undefined ? '' : input.progress
      if (typeof progress !== 'string' || progress.length > 2000) throw new Error('本次进展最多 2000 个字符')
      let activity = input.activityId ? this.get(input.activityId,draft) : null
      if (input.activityId && !activity) throw new Error('事项已不存在，请重新选择')
      const creating = !activity
      if (creating) activity = {
        id: this.newId(), name: typeof input.name === 'string' ? input.name.trim() : '', color:nextMatterColor(draft),
        startTime, endTime, status: 'completed', segments: [], sessions: [], nextSessionIndex: 0,
        todoId: null, createdAt: iso(time), updatedAt: iso(time), pauseReason: null
      }
      ensureSessions(activity)
      const session = {
        id: `${activity.id}:${activity.nextSessionIndex++}`,
        startTime, endTime, progress: progress.trim(), segments: [{start:startTime,end:endTime}]
      }
      validateActivity({...activity,startTime,endTime,status:'completed',segments:session.segments})
      // Use the visit ID to also check against existing visits of this same matter.
      assertNoOverlap({...activity,id:session.id,segments:session.segments},draft.activities)
      activity.sessions.push(session)
      activity.sessions.sort((a,b) => timestamp(a.startTime)-timestamp(b.startTime))
      syncSegments(activity)
      activity.startTime = iso(Math.min(timestamp(activity.startTime),timestamp(startTime)))
      if (activity.status === 'completed') activity.endTime = iso(Math.max(timestamp(activity.endTime),timestamp(endTime)))
      activity.updatedAt = iso(time)
      if (creating) draft.activities.push(activity)
    })
  }

  setProgress(activityId, sessionId, progress) {
    if (typeof progress !== 'string' || progress.length > 2000) throw new Error('本次进展最多 2000 个字符')
    return this.commit((draft,time)=>{
      const activity = this.get(activityId,draft)
      const session = activity && ensureSessions(activity).find(session=>session.id===sessionId)
      if (!session) throw new Error('本次记录已不存在')
      session.progress = progress.trim()
      activity.updatedAt = iso(time)
    })
  }

  nextSession(activityId, sessionId, progress) {
    if (typeof progress !== 'string' || progress.length > 2000) throw new Error('本次进展最多 2000 个字符')
    return this.commit((draft, time) => {
      const activity = this.current(draft)
      const session = activity && ensureSessions(activity).find(item => item.endTime === null)
      if (activity?.id !== activityId || !session || session.id !== sessionId) throw new Error('当前投入已变化，请核对后重试')
      if (time < timestamp(session.startTime)) throw new Error('系统时间早于本次开始时间，请检查系统时钟')
      session.progress = progress.trim()
      this.closeSegment(activity, time)
      appendSession(activity, time)
      activity.updatedAt = iso(time)
      assertNoOverlap(activity, draft.activities)
    })
  }

  updateMatter(id, input) {
    return this.commit((draft,time)=>{
      const activity = this.get(id,draft)
      if (!activity) throw new Error('事项已不存在')
      const name = typeof input.name === 'string' ? input.name.trim() : ''
      if (!name || name.length > 200) throw new Error('事项名称须为 1–200 个字符')
      if (!['running','paused','completed'].includes(input.status)) throw new Error('事项状态无效')
      activity.name = name
      if (input.status === 'completed' && activity.status !== 'completed') this.finish(activity,time)
      if (input.status === 'paused' && activity.status !== 'paused') {
        this.pauseRecord(activity,time,'manual')
        activity.endTime = null
      }
      if (input.status === 'running' && activity.status !== 'running') {
        const last = activity.segments.at(-1)
        if (last && time < timestamp(last.end)) throw new Error('系统时间早于上次记录')
        const foreground = this.current(draft)
        if (foreground) this.pauseRecord(foreground,time,'switch')
        appendSession(activity,time)
        activity.status = 'running'
        activity.endTime = null
        activity.pauseReason = null
      }
      activity.updatedAt = iso(time)
      assertNoOverlap(activity,draft.activities)
    })
  }

  editSession(input) {
    return this.commit((draft,time)=>{
      const activity = this.get(input.activityId,draft)
      const session = activity && ensureSessions(activity).find(session=>session.id===input.sessionId)
      if (!session) throw new Error('本次记录已不存在')
      if (session.endTime === null) throw new Error('请先暂停本次记录，再修改时间')
      const edited = {id:session.id, startTime:iso(timestamp(input.startTime)), endTime:iso(timestamp(input.endTime)),
        progress:String(input.progress || '').trim(), segments:input.segments.map(segment=>({start:iso(timestamp(segment.start)),end:iso(timestamp(segment.end))}))}
      validateActivity({...activity,id:edited.id,status:'completed',startTime:edited.startTime,endTime:edited.endTime,segments:edited.segments})
      if (timestamp(edited.endTime)>time || edited.segments.some(segment=>timestamp(segment.end)>time)) throw new Error('不能补录未来的活动时间')
      activity.sessions[activity.sessions.indexOf(session)] = edited
      activity.sessions.sort((a,b)=>timestamp(a.startTime)-timestamp(b.startTime))
      syncSegments(activity)
      activity.startTime = activity.sessions[0].startTime
      if (activity.endTime) activity.endTime = iso(Math.max(timestamp(activity.endTime),...activity.sessions.map(session=>timestamp(session.endTime))))
      activity.updatedAt = iso(time)
      assertNoOverlap(activity,draft.activities)
    })
  }

  deleteSession(activityId,sessionId) {
    return this.commit((draft,time)=>{
      const activity = this.get(activityId,draft)
      const session = activity && ensureSessions(activity).find(session=>session.id===sessionId)
      if (!session) throw new Error('本次记录已不存在')
      if (session.endTime === null) this.pauseRecord(activity,time,'manual')
      const parent = {...copy(activity),sessions:[],segments:[]}
      if (parent.status === 'running') { parent.status = 'paused'; parent.pauseReason = 'trash' }
      draft.trash ||= []
      draft.trash.push({id:this.newId(),kind:'session',deletedAt:iso(time),activity:parent,session:copy(session)})
      activity.sessions = activity.sessions.filter(session=>session.id!==sessionId)
      syncSegments(activity)
      activity.updatedAt = iso(time)
    })
  }

  restoreTrash(id) {
    return this.commit((draft,time) => {
      const entry = (draft.trash || []).find(entry => entry.id===id)
      if (!entry) throw new Error('回收站记录已不存在')
      const incoming = copy(entry.activity)
      if (entry.kind === 'session') incoming.sessions = [copy(entry.session)]
      ensureSessions(incoming)
      let target = this.get(incoming.id,draft)
      if (target) {
        ensureSessions(target)
        if (incoming.sessions.some(session => target.sessions.some(existing => existing.id===session.id))) throw new Error('同一记录已存在，无法重复恢复')
        target.sessions.push(...incoming.sessions)
        target.nextSessionIndex = Math.max(target.nextSessionIndex,incoming.nextSessionIndex)
      } else {
        target = incoming
        draft.activities.push(target)
      }
      target.sessions.sort((a,b) => timestamp(a.startTime)-timestamp(b.startTime))
      syncSegments(target)
      target.startTime = iso(Math.min(timestamp(target.startTime),...target.sessions.map(session => timestamp(session.startTime))))
      if (target.status === 'completed') target.endTime = iso(Math.max(timestamp(target.endTime),...target.sessions.map(session => timestamp(session.endTime))))
      target.updatedAt = iso(time)
      assertNoOverlap(target,draft.activities)
      draft.trash = draft.trash.filter(entry => entry.id!==id)
    })
  }

  deleteTrash(id) {
    return this.commit(draft => {
      if (!(draft.trash || []).some(entry => entry.id===id)) throw new Error('回收站记录已不存在')
      draft.trash = draft.trash.filter(entry => entry.id!==id)
    })
  }

}
module.exports = { TrackingModel }
