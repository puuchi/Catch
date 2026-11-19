const { chooseMatterColor } = require('./colors')
const iso = time => new Date(time).toISOString()
const copy = value => JSON.parse(JSON.stringify(value))

function timestamp(value) {
  if (typeof value !== 'string' || !Number.isFinite(Date.parse(value))) throw new Error('时间格式无效')
  return Date.parse(value)
}

function validateActivity(activity) {
  if (!activity || typeof activity.id !== 'string' || !activity.id) throw new Error('记录 ID 无效')
  if (typeof activity.name !== 'string' || !activity.name.trim() || activity.name.length > 200) throw new Error('活动名称须为 1–200 个字符')
  if (activity.color !== undefined && !/^#[\da-f]{6}$/i.test(activity.color)) throw new Error('事项颜色无效')
  if (!['running', 'paused', 'completed'].includes(activity.status)) throw new Error('活动状态无效')
  const start = timestamp(activity.startTime)
  const end = activity.endTime === null ? Infinity : timestamp(activity.endTime)
  if (end < start || (activity.status === 'completed') !== (activity.endTime !== null)) throw new Error('结束时间必须晚于或等于开始时间')
  if (!Array.isArray(activity.segments)) throw new Error('有效时间段无效')
  let previousEnd = start
  let openCount = 0
  for (const [index, segment] of activity.segments.entries()) {
    const from = timestamp(segment.start)
    const to = segment.end === null ? Infinity : timestamp(segment.end)
    if (from < previousEnd || to < from || to > end) throw new Error('有效时间段必须按顺序排列，且位于开始和结束时间之间')
    if (segment.end === null) {
      openCount++
      if (activity.status !== 'running' || index !== activity.segments.length - 1) throw new Error('只有进行中的最后一段可以没有结束时间')
    }
    previousEnd = to
  }
  if ((activity.status === 'running' && openCount !== 1) || openCount > 1) throw new Error('进行中活动必须有一个开放时间段')
}

function validateDocument(data) {
  if (!data || ![1, 2].includes(data.version) || !Array.isArray(data.activities)) throw new Error('无法识别活动数据版本，请保留文件并使用兼容版本')
  if (data.checkpointAt !== null) timestamp(data.checkpointAt)
  const ids = new Set()
  let active = 0
  for (const activity of data.activities) {
    validateActivity(activity)
    if (ids.has(activity.id)) throw new Error('存在重复记录 ID')
    ids.add(activity.id)
    if (activity.status === 'running' || (data.version === 1 && activity.status === 'paused')) active++
  }
  if (active > 1) throw new Error('存在多项前台计时活动')
  if (data.sessionVersion !== undefined && data.sessionVersion !== 1) throw new Error('无法识别本次记录的数据版本')
  const sessionIds = new Set()
  for (const activity of data.activities) {
    if (!Array.isArray(activity.sessions)) continue
    for (const session of activity.sessions) {
      if (typeof session.id !== 'string' || !session.id || sessionIds.has(session.id)) throw new Error('本次记录 ID 无效或重复')
      sessionIds.add(session.id)
      if (typeof session.progress !== 'string' || session.progress.length > 2000) throw new Error('本次进展最多 2000 个字符')
      validateActivity({...activity, id:session.id, startTime:session.startTime, endTime:session.endTime,
        status:session.endTime === null ? 'running' : 'completed', segments:session.segments})
      if (session.endTime === null && activity.status !== 'running') throw new Error('暂停事项不能包含正在计时的记录')
    }
    const flat = activity.sessions.flatMap(session=>session.segments).sort((a,b)=>timestamp(a.start)-timestamp(b.start))
    if (JSON.stringify(flat) !== JSON.stringify(activity.segments)) throw new Error('本次记录与有效时间段不一致')
  }
  const intervals = data.activities.flatMap(activity => activity.segments.map(segment => ({
    start: timestamp(segment.start), end: segment.end === null ? Infinity : timestamp(segment.end), name: activity.name
  }))).filter(interval => interval.end > interval.start).sort((a, b) => a.start - b.start)
  for (let index = 1; index < intervals.length; index++) {
    if (intervals[index].start < intervals[index - 1].end) throw new Error('不同事项的有效时间段重叠')
  }
  if (data.trash !== undefined && !Array.isArray(data.trash)) throw new Error('活动回收站格式无效')
  const trashIds = new Set()
  for (const entry of data.trash || []) {
    if (!entry || !['matter','session'].includes(entry.kind)) throw new Error('无法识别活动回收站版本')
    if (typeof entry.id !== 'string' || !entry.id || trashIds.has(entry.id)) throw new Error('回收站记录 ID 无效或重复')
    trashIds.add(entry.id)
    timestamp(entry.deletedAt)
    validateActivity(entry.activity)
    if (entry.activity.status === 'running') throw new Error('回收站事项必须停止计时')
    const activity = entry.kind === 'matter' ? entry.activity : {
      ...entry.activity, startTime:entry.session.startTime, endTime:entry.session.endTime,
      status:'completed', sessions:[entry.session], segments:entry.session.segments
    }
    validateDocument({version:2,sessionVersion:1,checkpointAt:null,activities:[activity]})
  }
}

function nextMatterColor(data) {
  return chooseMatterColor([...data.activities.map(activity => activity.color),...(data.trash || []).map(entry => entry.activity.color)])
}

function assertNoOverlap(candidate, others) {
  for (const other of others) {
    if (other.id === candidate.id) continue
    for (const first of candidate.segments) {
      for (const second of other.segments) {
        const start = Math.max(timestamp(first.start), timestamp(second.start))
        const end = Math.min(first.end === null ? Infinity : timestamp(first.end), second.end === null ? Infinity : timestamp(second.end))
        if (start < end) throw new Error(`时间冲突：与「${other.name}」的有效时间段重叠，请调整时间`)
      }
    }
  }
}


module.exports = { iso, copy, timestamp, validateActivity, validateDocument, nextMatterColor, assertNoOverlap }
