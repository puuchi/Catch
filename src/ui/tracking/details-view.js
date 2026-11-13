const ActivitySessions=require('../../domain/tracking/sessions')
const {matterColor,humanDuration,duration,sessionDays,timelineEntries,dayBounds}=require('../../domain/tracking/time')
module.exports=function renderDetails({matter,anchor,selectedDay,locate,byId,paintStrip,timeLabel,onEdit}) {
    byId('matterName').textContent = matter.name
    byId('matterName').style.setProperty('--matter-color',matterColor(matter.id,matter.color).hex)
    byId('matterStatus').textContent = {running:'进行中',paused:'已暂停',completed:'已完成'}[matter.status]
    byId('matterStatus').className = `timeline-status ${matter.status}`
    byId('matterTotal').textContent = humanDuration(duration(matter))
    byId('matterVisits').textContent = `${ActivitySessions.sessionsOf(matter).length} 次`
    const list = byId('matterSessions')
    const scrollTop = byId('matterDetailBody').scrollTop
    list.replaceChildren()
    for (const day of sessionDays(matter)) {
      const group = document.createElement('section')
      group.className = 'detail-day'
      const label = document.createElement('h3')
      label.textContent = day.replace(/-/g,' / ')
      group.append(label)
      const entries = timelineEntries([matter],day).sort((a,b)=>Date.parse(a.startTime)-Date.parse(b.startTime))
      for (const entry of entries) {
        const row = document.createElement('article')
        row.className = 'detail-session'
        row.dataset.sessionId = entry.sessionId
        row.dataset.day = day
        paintStrip(row,matter,duration(entry))
        const title = document.createElement('strong')
        title.textContent = `${entry.sessionIndex > 0 ? '↩ ' : ''}${entry.progress || matter.name}`
        title.title = entry.progress || matter.name
        const metadata = document.createElement('div')
        metadata.className = 'detail-session-meta'
        metadata.textContent = `${timeLabel(entry.startTime,day)}–${entry.endTime ? timeLabel(entry.endTime,day) : '现在'} · ${humanDuration(duration(entry,Date.now(),dayBounds(day)))}`
        const edit = document.createElement('button')
        edit.className = 'session-edit'
        edit.textContent = '✎'
        edit.title = '编辑'
        edit.setAttribute('aria-label','编辑本次记录')
        edit.addEventListener('click',async()=>{
          await onEdit(matter.id,entry.sessionId)
        })
        const actions = document.createElement('div')
        actions.className = 'detail-session-actions'
        actions.append(edit)
        row.append(title,metadata,actions)
        group.append(row)
      }
      list.append(group)
    }
    byId('matterDetailBody').scrollTop = scrollTop
    if (anchor) {
      const target = [...list.querySelectorAll('.detail-session')].find(row=>row.dataset.sessionId===anchor && row.dataset.day===selectedDay)
      target?.classList.add('detail-origin')
      if (locate) target?.scrollIntoView({block:'nearest'})
    }
}
