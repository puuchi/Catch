const ActivityTime = require('../../domain/tracking/time')
const ActivitySessions = require('../../domain/tracking/sessions')
const icons = require('../shared/icons')
const {foreground, statusText} = require('./foreground')
const {clearFormErrors} = require('../shared/form-validation')

module.exports = function mount(api) {

  const byId = id => document.getElementById(id)
  const { localDate, dayBounds, duration, clock, humanDuration, heatLevel, timelineEntries, matterColor, durationOpacity, untrackedGaps, sessionDays } = ActivityTime
  let state = { activities: [] }
  let busy = false
  let page = 'todos'
  let editing = null
  let editingRevision = 0
  let editorScope = 'new'
  let editingSessionId = null
  let editorEndDayOffset = 0
  let editorEndDayExplicit = false
  let focusedActivityId = null
  let detailActivityId = null
  let detailAnchor = null
  let progressSessionId = null
  let progressDirty = false
  let progressEditing = false
  const dismissedNotices = new Set()
  let visibleNotice = null
  let deleteId = null
  let firstLoad = true
  let foregroundId = null
  let trashReturnPage = 'todos'
  let pendingMatterDelete = null
  let timelineRenderDay = localDate()
  const editor = byId('activityEditor')
  const menu = byId('timelineMenu')
  const menuEdit = byId('timelineMenuEdit')
  const menuDelete = byId('timelineMenuDelete')
  let menuActivityId = null
  let menuTrigger = null
  let editorReturnActivityId = null
  let editorReturnEntryKey = null
  let menuEntryKey = null
  let menuAnchorTop = 0
  const dateInput = byId('timelineDate')
  dateInput.value = localDate()
  byId('deleteMatter').append(icons.create('trash'))
  byId('deleteSelectedMatter').append(icons.create('trash'))
  byId('clearFocus').replaceChildren(icons.create('close'))
  for (const [id, icon] of Object.entries({pauseActivity:'pause',endActivity:'stop',nextProgress:'next',backgroundIcon:'pause'})) {
    byId(id).append(icons.create(icon))
  }

  function current() { return state.activities.find(activity => activity.status === 'running') }

  function paused() { return state.activities.filter(activity => activity.status === 'paused') }

  function selected() { return foreground(state.activities, foregroundId) }

  function updateActions() {
    document.querySelectorAll(actionButtons).forEach(button => { button.disabled = busy || Boolean(state.error) })
    byId('nextProgress').disabled = busy || Boolean(state.error) || !current()
  }

  function fitCurrentTimer() {
    const timer = byId('currentTimer')
    if (!timer.clientWidth) return
    timer.style.fontSize = ''
    const size = parseFloat(getComputedStyle(timer).fontSize)
    if (timer.scrollWidth > timer.clientWidth) {
      timer.style.fontSize = `${Math.max(10,Math.floor(size*timer.clientWidth/timer.scrollWidth*.98))}px`
    }
  }
  let timerRowWidth = 0
  new ResizeObserver(entries => {
    const width = entries[0].contentRect.width
    if (width !== timerRowWidth) { timerRowWidth = width; fitCurrentTimer() }
  }).observe(byId('currentTimer').parentElement)
  window.addEventListener('resize',fitCurrentTimer)

  const actionButtons = '#startActivityForm button, #progressForm button, #pauseActivity, #endActivity, #restartActivity, #saveActivity, .todo-start, .background-actions button'

  function message(text, { notice = false } = {}) {
    const hidden = !text || notice && dismissedNotices.has(text)
    visibleNotice = notice && !hidden ? text : null
    byId('activityMessageText').textContent = hidden ? '' : text
    byId('activityMessage').hidden = hidden
    byId('dismissActivityMessage').hidden = !visibleNotice
  }

  byId('dismissActivityMessage').addEventListener('click', () => {
    if (!visibleNotice) return
    dismissedNotices.add(visibleNotice)
    message('')
    document.querySelector('[data-page].selected')?.focus({preventScroll:true})
  })

  function selectPage(next) {
    closeLegend()
    closeMenu(false)
    deleteId = null
    page = next
    document.querySelectorAll('.module-page').forEach(element => { element.hidden = element.id !== `page-${next}` })
    document.querySelectorAll('[data-page]').forEach(button => {
      button.classList.toggle('selected', button.dataset.page === next)
      button.setAttribute('aria-selected', String(button.dataset.page === next))
    })
    byId('todoTrashBtn').setAttribute('aria-pressed',String(next === 'trash'))
    if (next === 'timeline') { renderTimeline(); if (detailActivityId) renderDetails() }
  }

  function setState(next) {
    if ((next.revision || 0) < (state.revision || 0)) return
    state = next
    message(state.error || state.notice, { notice: !state.error && Boolean(state.notice) })
    if (firstLoad && (current() || paused().length)) selectPage('current')
    firstLoad = false
    renderCurrent()
    if (page === 'timeline') { renderTimeline(); if (detailActivityId) renderDetails() }
  }

  async function command(type, payload = {}) {
    if (busy) return false
    busy = true
    document.querySelectorAll(actionButtons).forEach(button => { button.disabled = true })
    try {
      const editingCommand = editor.open && ['edit','session-edit','session-add','matter-edit'].includes(type)
      const response = await api.activityCommand({ type, payload, requestId: crypto.randomUUID(), expectedRevision: editingCommand ? editingRevision : state.revision || 0 })
      if (response.state) setState(response.state)
      if (editingCommand) editingRevision = state.revision || 0
      if (!response.ok) throw new Error(response.error)
      return true
    } catch (error) {
      message(error.message)
      if (editor.open) {
        byId('editorError').textContent = error.message
        byId('editorError').scrollIntoView({ block: 'nearest' })
      }
      return false
    } finally {
      busy = false
      updateActions()
    }
  }

  function renderCurrent() {
    const activity = selected()
    foregroundId = activity?.id || null
    const running = activity?.status === 'running'
    const completed = activity?.status === 'completed'
    const background = paused().filter(item => item.id !== foregroundId)
    const session = activity && ActivitySessions.sessionsOf(activity).at(-1)
    byId('currentName').textContent = activity?.name || '暂无前台事项'
    byId('currentName').title = activity?.name || ''
    byId('currentStatus').setAttribute('aria-label', statusText(activity))
    byId('currentStatus').title = statusText(activity)
    byId('currentStatusText').textContent = statusText(activity)
    byId('currentStatus').className = activity?.status || ''
    byId('navActivityDot').className = running ? 'running' : paused().length ? 'paused' : ''
    byId('currentStarted').hidden = !session || completed
    byId('currentStartedTime').textContent = session ? `${new Date(session.startTime).toLocaleTimeString([], {hour:'2-digit',minute:'2-digit',hour12:false})} 开始` : ''
    byId('currentStartedTime').dateTime = session?.startTime || ''
    byId('currentTimer').textContent = activity ? clock(duration(activity)) : '00:00:00'
    byId('currentHint').hidden = true
    byId('progressForm').hidden = !session
    const progressInput = byId('currentProgress')
    if (progressSessionId !== (session?.id || null)) {
      progressInput.value = session?.progress || ''
      progressDirty = false
      progressEditing = !session?.progress
    } else if (!progressDirty && document.activeElement !== progressInput) {
      progressInput.value = session?.progress || ''
    }
    progressSessionId = session?.id || null
    renderProgress(session)
    byId('pauseActivity').hidden = !activity || completed
    byId('pauseActivity').replaceChildren(icons.create(running ? 'pause' : 'play'))
    byId('pauseActivity').title = running ? '暂停' : '继续'
    byId('pauseActivity').setAttribute('aria-label', running ? '暂停当前事项' : '继续当前事项')
    byId('endActivity').hidden = !activity || completed
    byId('nextProgress').hidden = !activity || completed
    byId('completedSummary').hidden = !completed
    byId('currentVisits').textContent = activity ? `累计 ${ActivitySessions.sessionsOf(activity).length} 次投入` : ''
    byId('backgroundActivities').hidden = !background.length
    byId('backgroundCount').textContent = String(background.length)
    byId('backgroundToggle').setAttribute('aria-label', `后台暂停事项，${background.length} 项`)
    require('./background-list')(byId('pausedActivityList'),background,command)
    fitCurrentTimer()
    updateActions()
  }

  function renderProgress(session) {
    const completed = Boolean(session?.progress) && !progressEditing
    byId('currentProgress').hidden = completed
    byId('progressDisplay').hidden = !completed
    byId('progressForm').classList.toggle('is-complete',completed)
    byId('progressText').textContent = session?.progress || ''
  }

  function timeRange(start, end) {
    const range = document.createElement('div')
    range.className = 'timeline-range'
    for (const [boundary,value] of [['start',start],['end',end]]) {
      const time = document.createElement('time')
      time.className = `timeline-${boundary}`
      time.textContent = value ? timeLabel(value,dateInput.value) : '现在'
      if (value) time.dateTime = value
      range.append(time)
    }
    return range
  }

  function visibleGaps(now) {
    return untrackedGaps(state.activities,dateInput.value,now)
      .filter(gap => Date.parse(gap.endTime)-Date.parse(gap.startTime)>=10000)
  }

  function timeLabel(time, selectedDay) {
    const date = new Date(time)
    const timeText = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })
    return localDate(date) === selectedDay ? timeText : `${date.getMonth() + 1}/${date.getDate()} ${timeText}`
  }

  function paintStrip(row, activity, milliseconds) {
    row.dataset.heat = heatLevel(milliseconds)
    row.style.setProperty('--matter-rgb', matterColor(activity.id,activity.color).rgb)
    row.style.setProperty('--visit-opacity', durationOpacity(milliseconds))
  }

  function applyFocus() {
    const visits = timelineEntries(state.activities.filter(item => item.id === focusedActivityId),dateInput.value)
    if (!visits.length) focusedActivityId = null
    const rows = [...byId('timelineList').querySelectorAll('.timeline-item')]
    for (const row of rows) {
      const selected = row.dataset.activityId === focusedActivityId
      row.classList.toggle('is-focused', Boolean(focusedActivityId) && selected)
      row.classList.toggle('is-dimmed', Boolean(focusedActivityId) && !selected)
      row.setAttribute('aria-pressed', String(Boolean(focusedActivityId) && selected))
    }
    const matter = state.activities.find(item => item.id === focusedActivityId)
    byId('focusSummary').hidden = !matter
    if (matter) {
      byId('focusSummaryText').textContent = matter.name
      byId('focusSummaryText').title = matter.name
      byId('focusSummaryStats').textContent = `当日 ${visits.length} 次投入 · 累计 ${humanDuration(duration(matter,Date.now(),dayBounds(dateInput.value)))}`
      byId('focusSummary').style.setProperty('--focus-color',matterColor(matter.id,matter.color).hex)
    }
  }

  function toggleFocus(id) {
    focusedActivityId = focusedActivityId === id ? null : id
    applyFocus()
  }

  function gapRow(gap) {
    const row = document.createElement('div')
    row.className = 'timeline-gap'
    row.dataset.trailing = String(Boolean(gap.trailing))
    row.dataset.startTime = gap.startTime
    const text = document.createElement('span')
    text.textContent = '未计时'
    const total = document.createElement('span')
    total.className = 'gap-duration'
    total.textContent = humanDuration(Date.parse(gap.endTime)-Date.parse(gap.startTime))
    const actions = document.createElement('span')
    actions.className = 'gap-actions'
    const add = document.createElement('button')
    add.type = 'button'
    add.className = 'gap-add'
    add.append(icons.create('plus'))
    add.title = '补录'
    add.setAttribute('aria-label','补录未计时区间')
    add.addEventListener('click',() => openEditor(null,null,'new',gap))
    actions.append(text,total)
    row.append(timeRange(gap.startTime,gap.endTime),actions,add)
    return row
  }

  function renderTimeline() {
    if (!dateInput.value) return
    timelineRenderDay = localDate()
    const bounds = dayBounds(dateInput.value)
    const now = Date.now()
    const entries = timelineEntries(state.activities,dateInput.value,now)
    updateDayTotal(bounds,now)
    const list = byId('timelineList')
    list.replaceChildren()
    if (!entries.length) {
      closeMenu(false)
      const empty = document.createElement('p')
      empty.className = 'empty-hint'
      empty.textContent = '暂无记录'
      list.append(empty)
    }
    const direction = dateInput.value === localDate() ? -1 : 1
    const logs = [...entries,...visibleGaps(now)]
      .sort((a,b) => direction * (Date.parse(a.startTime)-Date.parse(b.startTime)))
    for (const activity of logs) {
      if (activity.gap) { list.append(gapRow(activity)); continue }
      const row = document.createElement('article')
      row.className = `timeline-item ${activity.status}`
      row.dataset.activityId = activity.id
      row.dataset.entryKey = activity.entryKey
      row.dataset.startTime = activity.startTime
      row.tabIndex = 0
      row.setAttribute('role','button')
      row.setAttribute('aria-label',`${activity.name}，聚焦事项`)
      paintStrip(row,activity,duration(activity,now))
      const range = timeRange(activity.startTime,activity.endTime)
      const title = document.createElement('div')
      title.className = 'timeline-name'
      title.title = activity.name
      const name = document.createElement('span')
      name.className = 'timeline-title'
      name.textContent = activity.name
      if (activity.sessionIndex > 0) {
        const mark = document.createElement('span')
        mark.className = 'timeline-return-mark'
        mark.title = '再次投入'
        mark.setAttribute('aria-label','再次投入')
        mark.append(icons.create('return'))
        title.append(mark)
      }
      title.append(name)
      const total = document.createElement('div')
      total.className = 'muted timeline-duration'
      total.textContent = humanDuration(duration(activity,now,bounds))
      const heading = document.createElement('div')
      heading.className = 'timeline-heading'
      heading.append(title,total)
      const metadata = document.createElement('div')
      metadata.className = 'timeline-meta'
      if (activity.progress) {
        const progress = document.createElement('span')
        progress.className = 'timeline-progress'
        progress.textContent = activity.progress
        progress.title = progress.textContent
        metadata.append(progress)
      }
      if (activity.status !== 'completed') {
        const status = document.createElement('span')
        status.className = `timeline-status ${activity.status}`
        status.textContent = activity.status === 'running' ? '进行中' : '已暂停'
        metadata.append(status)
      }
      const toggle = document.createElement('button')
      toggle.className = 'timeline-menu-toggle'
      toggle.textContent = '···'
      toggle.title = '操作'
      toggle.setAttribute('aria-label',`活动操作：${activity.name}`)
      toggle.setAttribute('aria-haspopup','menu')
      toggle.setAttribute('aria-expanded','false')
      toggle.setAttribute('aria-controls','timelineMenu')
      toggle.addEventListener('click',event => {
        event.stopPropagation()
        if (menuEntryKey === activity.entryKey) closeMenu(true)
        else showMenu(activity.id,activity.entryKey,toggle)
      })
      toggle.addEventListener('keydown',event => {
        event.stopPropagation()
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
          event.preventDefault()
          showMenu(activity.id,activity.entryKey,toggle,event.key === 'ArrowUp')
        }
      })
      row.addEventListener('click',() => toggleFocus(activity.id))
      row.addEventListener('keydown',event => {
        if (event.target !== row) return
        if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); toggleFocus(activity.id) }
      })
      row.append(range,heading,metadata,toggle)
      list.append(row)
    }
    applyFocus()
    if (menuActivityId) {
      menuTrigger = findMenuTrigger(menuActivityId,menuEntryKey)
      if (menuTrigger) { menuTrigger.setAttribute('aria-expanded','true'); positionMenu() }
      else closeMenu(false)
    }
  }

  function renderDetails(locate = false) {
    const matter=state.activities.find(item=>item.id===detailActivityId)
    if(!matter){showOverview();return}
    require('./details-view')({matter,anchor:detailAnchor,selectedDay:dateInput.value,locate,byId,paintStrip,timeLabel,
      onEdit:async(id,sessionId)=>{
        const latest=state.activities.find(item=>item.id===id)
        const session=ActivitySessions.sessionsOf(latest).find(item=>item.id===sessionId)
        if(session.endTime===null && !await command('pause'))return
        openEditor(state.activities.find(item=>item.id===id),sessionId,'session')
      }})
  }

  function showDetails(id,sessionId) {
    resetMatterDelete()
    detailActivityId = id
    detailAnchor = sessionId
    byId('timelineOverview').hidden = true
    byId('matterDetails').hidden = false
    renderDetails(true)
    byId('backToTimeline').focus({preventScroll:true})
  }

  function showOverview() {
    detailActivityId = null
    byId('matterDetails').hidden = true
    byId('timelineOverview').hidden = false
    renderTimeline()
  }

  function findMenuTrigger(id, entryKey) {
    const rows = [...byId('timelineList').children]
    const row = rows.find(row => entryKey && row.dataset.entryKey === entryKey) || rows.find(row => row.dataset.activityId === id)
    return row?.querySelector('.timeline-menu-toggle')
  }

  function updateGaps(now) {
    const list = byId('timelineList')
    list.querySelectorAll('.timeline-gap').forEach(row => row.remove())
    const direction = dateInput.value === localDate() ? -1 : 1
    for (const gap of visibleGaps(now)) {
      const next = [...list.children].find(row => row.dataset.startTime &&
        direction * (Date.parse(row.dataset.startTime) - Date.parse(gap.startTime)) > 0)
      list.insertBefore(gapRow(gap),next || null)
    }
  }

  function closeMenu(restoreFocus) {
    const trigger = menuTrigger
    trigger?.setAttribute('aria-expanded', 'false')
    menu.hidden = true
    menuActivityId = null
    menuEntryKey = null
    menuTrigger = null
    deleteId = null
    menuDelete.setAttribute('aria-label','删除本次记录')
    menuDelete.title = '删除'
    menuDelete.classList.remove('danger')
    if (restoreFocus && trigger?.isConnected) trigger.focus()
  }

  function positionMenu() {
    const bounds = menuTrigger.getBoundingClientRect()
    menuAnchorTop = bounds.top
    menu.style.left = `${Math.max(8, Math.min(bounds.right - menu.offsetWidth, innerWidth - menu.offsetWidth - 8))}px`
    menu.style.top = `${Math.max(8, Math.min(bounds.bottom + 3, innerHeight - menu.offsetHeight - 8))}px`
  }

  function showMenu(id, entryKey, trigger, focusLast = false) {
    closeMenu(false)
    menuActivityId = id
    menuEntryKey = entryKey
    menuTrigger = trigger
    trigger.setAttribute('aria-expanded', 'true')
    menu.hidden = false
    positionMenu()
    menuEdit.title = '详情'
    ;(focusLast ? menuDelete : menuEdit).focus({preventScroll:true})
  }

  menu.addEventListener('keydown', event => {
    if (event.key === 'Escape') { event.preventDefault(); closeMenu(true); return }
    if (event.key === 'Tab') { closeMenu(true); return }
    if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault()
      const items = [menuEdit, menuDelete]
      const index = items.indexOf(document.activeElement)
      let next = (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length
      if (event.key === 'Home') next = 0
      if (event.key === 'End') next = 1
      items[next].focus({preventScroll:true})
    }
  })
  document.addEventListener('pointerdown', event => {
    if (!menu.hidden && !menu.contains(event.target) && !menuTrigger?.contains(event.target)) closeMenu(false)
  })
  document.addEventListener('focusin', event => {
    if (!menu.hidden && !menu.contains(event.target) && event.target !== menuTrigger) closeMenu(false)
  })
  byId('timelineList').addEventListener('scroll', () => {
    if (!menu.hidden && menuTrigger && Math.abs(menuTrigger.getBoundingClientRect().top - menuAnchorTop) > .5) closeMenu(false)
  })
  window.addEventListener('resize', () => closeMenu(false))
  menuEdit.addEventListener('click', () => {
    const id = menuActivityId
    const entryKey = menuEntryKey
    closeMenu(false)
    showDetails(id,entryKey)
  })
  menuDelete.addEventListener('click',async()=>{
    const id = menuActivityId
    const sessionId = menuEntryKey
    if (!id) return
    if (deleteId !== sessionId) {
      deleteId = sessionId
      menuDelete.setAttribute('aria-label','确认删除本次记录')
      menuDelete.title = '确认删除'
      menuDelete.classList.add('danger')
      return
    }
    const rows = [...byId('timelineList').querySelectorAll('.timeline-item')]
    const index = rows.findIndex(row => row.dataset.entryKey === sessionId)
    const neighbor = rows[index + 1] || rows[index - 1]
    const neighborId = neighbor?.dataset.activityId
    const neighborKey = neighbor?.dataset.entryKey
    closeMenu(false)
    const removed = await command('session-delete',{activityId:id,sessionId})
    const target = removed ? findMenuTrigger(neighborId,neighborKey) : findMenuTrigger(id,sessionId)
    ;(target || byId('addActivity')).focus({preventScroll:true})
  })
  byId('clearFocus').addEventListener('click',()=>{ focusedActivityId=null; applyFocus() })
  byId('timelineOverview').addEventListener('click',event=>{
    if (!event.target.closest('.timeline-item, button, input')) { focusedActivityId=null; applyFocus() }
  })
  byId('backToTimeline').addEventListener('click',()=>{
    const id = detailActivityId
    showOverview()
    findMenuTrigger(id,detailAnchor)?.focus()
  })
  byId('editMatter').addEventListener('click',()=>openEditor(state.activities.find(item=>item.id===detailActivityId),null,'matter'))
  byId('deleteMatter').addEventListener('click',() => trashMatter(detailActivityId,byId('deleteMatter')))
  byId('deleteSelectedMatter').addEventListener('click',() => trashMatter(byId('recordActivity').value,byId('deleteSelectedMatter')))
  byId('todoTrashBtn').addEventListener('click',() => {
    if (page === 'trash') { selectPage(trashReturnPage); return }
    trashReturnPage = page
    selectPage('trash')
    window.dispatchEvent(new Event('recycle-open'))
  })
  byId('trashBack').addEventListener('click',() => selectPage(trashReturnPage))


  byId('editProgress').addEventListener('click',() => {
    progressEditing = true
    renderProgress()
    const input = byId('currentProgress')
    input.focus()
    input.setSelectionRange(input.value.length,input.value.length)
  })
  byId('currentProgress').addEventListener('input',() => {
    progressDirty = true

  })
  for (const [input, form] of [['currentProgress','progressForm'],['activityName','startActivityForm']]) {
    byId(input).addEventListener('keydown',event => {
      if (event.key !== 'Enter' || event.isComposing || event.keyCode === 229) return
      event.preventDefault()
      if (!event.repeat) byId(form).requestSubmit()
    })
  }
  byId('progressForm').addEventListener('submit',async event=>{
    event.preventDefault()
    const activity = selected()
    const sessionId = progressSessionId
    if (!activity || !sessionId) return
    const progress = byId('currentProgress').value
    progressDirty = true
    // Keep the visit ID and draft attached to the original session during IPC.
    if (await command('progress',{activityId:activity.id,sessionId,progress})) {
      if (progressSessionId === sessionId && byId('currentProgress').value === progress) {
        progressDirty = false
        byId('currentProgress').value = progress.trim()
        progressEditing = !progress.trim()
        renderProgress({progress:progress.trim()})
        if (!progressEditing) byId('progressDisplay').focus({preventScroll:true})
      }
    }
  })
  byId('nextProgress').addEventListener('click',async event => {
    if (event.detail > 1) return
    const activity = current()
    const sessionId = progressSessionId
    if (!activity || !sessionId || busy) return
    progressDirty = true
    const input = byId('currentProgress')
    input.disabled = true
    const started = await command('next-session',{activityId:activity.id,sessionId,progress:input.value})
    input.disabled = false
    if (started) input.focus()
  })

  function updateDayTotal(bounds, now) {
    const total = state.activities.reduce((sum, activity) => sum + duration(activity, now, bounds), 0)
    const prefix = dateInput.value === localDate() ? '今日已记录' : '当日已记录'
    byId('dayTotal').textContent = `${prefix} ${humanDuration(total)}`
  }

  function localInput(value) {
    const date = new Date(value)
    return `${localDate(date)}T${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}:${String(date.getSeconds()).padStart(2, '0')}`
  }

  function inputTime(input) {
    let value = input.value.trim().replace(' ', 'T')
    if (input.id === 'editActivityStart' || input.id === 'editActivityEnd') {
      const day = input.id === 'editActivityEnd' ? editorEndDate() : byId('editActivityDate').value
      value = `${day}T${value}`
    }
    if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/.test(value)) value += ':00'
    if (input.dataset.originalTime && value === localInput(input.dataset.originalTime)) return input.dataset.originalTime
    const date = new Date(value)
    if (!Number.isFinite(+date) || localInput(date) !== value) throw new Error('请填写有效的日期与时间')
    return date.toISOString()
  }

  function editorSegments() {
    if (editorScope === 'new') return [{start:inputTime(byId('editActivityStart')),end:inputTime(byId('editActivityEnd'))}]
    return ActivitySessions.resizeSessionSegments(editing,
      inputTime(byId('editActivityStart')),inputTime(byId('editActivityEnd')))
  }

  function updateEditorDuration() {
    const startText = byId('editActivityStart').value
    const endText = byId('editActivityEnd').value
    const timePattern = /^\d{2}:\d{2}(:\d{2})?$/
    if (!editorEndDayExplicit && timePattern.test(startText) && timePattern.test(endText)) {
      editorEndDayOffset = startText.padEnd(8,':00') > endText.padEnd(8,':00') ? 1 : 0
      updateEndDay()
    }
    try {
      const start = Date.parse(inputTime(byId('editActivityStart')))
      const end = byId('editActivityEnd').value ? Date.parse(inputTime(byId('editActivityEnd'))) : Infinity
      if (end < start) throw new Error('Invalid bounds')
      const segments = editorSegments()
      let previousEnd = start
      let total = 0
      for (const segment of segments) {
        const from = Date.parse(segment.start)
        const to = Date.parse(segment.end)
        if (from < previousEnd || to < from || to > end) throw new Error('Invalid segment')
        total += to - from
        previousEnd = to
      }
      byId('editActivityDuration').textContent = humanDuration(total).replace(/\s/g,'')
    } catch {
      byId('editActivityDuration').textContent = '—'
    }
  }

  function useNow(input) {
    if (input.id === 'editActivityStart' || input.id === 'editActivityEnd') setEditorTime(input,new Date())
    else input.value = localInput(Date.now())
    delete input.dataset.originalTime
    input.dispatchEvent(new Event('input', { bubbles: true }))
    input.dispatchEvent(new Event('change', { bubbles: true }))
    input.focus()
  }

  function editorEndDate() {
    const date = new Date(`${byId('editActivityDate').value}T12:00:00`)
    date.setDate(date.getDate()+editorEndDayOffset)
    return localDate(date)
  }

  function updateEndDay() {
    byId('endDayToggle').textContent = editorEndDayOffset === 0 ? '当天' : editorEndDayOffset === 1 ? '次日' : `${editorEndDayOffset > 0 ? '+' : ''}${editorEndDayOffset}天`
    byId('endDayToggle').title = '结束日期'
    byId('endDayToggle').hidden = editorScope === 'matter' || editorEndDayOffset === 0
  }

  function setEditorTime(input,date) {
    const day = localDate(date)
    if (input.id === 'editActivityStart') byId('editActivityDate').value = day
    else {
      editorEndDayOffset = Math.round((Date.parse(`${day}T00:00:00Z`)-Date.parse(`${byId('editActivityDate').value}T00:00:00Z`))/86400000)
      editorEndDayExplicit = true
    }
    input.value = localInput(date).split('T')[1]
    updateEndDay()
  }

  const {closeTimePicker} = require('./time-picker')({byId,editor,inputTime,localInput,setEditorTime,useNow})

  function updateMatterSelection() {
    const creating = editorScope === 'new' && !byId('recordActivity').value
    const name = byId('editActivityName')
    name.closest('.editor-field').hidden = !creating && editorScope !== 'matter'
    name.disabled = !creating && editorScope !== 'matter'
    name.required = creating || editorScope === 'matter'
    byId('deleteSelectedMatter').hidden = editorScope !== 'new' || !byId('recordActivity').value
    resetMatterDelete()
  }

  function resetMatterDelete() {
    pendingMatterDelete = null
    for (const id of ['deleteMatter','deleteSelectedMatter']) {
      byId(id).classList.remove('danger')
      byId(id).title = '删除事项'
    }
  }

  async function trashMatter(id,button) {
    if (!id) return
    if (pendingMatterDelete !== id) {
      resetMatterDelete()
      pendingMatterDelete = id
      button.classList.add('danger')
      button.title = '确认删除'
      return
    }
    if (await command('remove',{id})) {
      resetMatterDelete()
      if (editor.open) editor.close()
      if (detailActivityId === id) showOverview()
    }
  }

  function openEditor(activity = null, entryKey = null, scope = 'new', gap = null) {
    clearFormErrors(byId('activityEditorForm'))
    editingRevision = state.revision || 0
    closeMenu(false)
    closeTimePicker()
    closeLegend()
    editorReturnActivityId = activity?.id || null
    editorReturnEntryKey = entryKey
    editorScope = scope
    editingSessionId = scope === 'session' ? entryKey : null
    const session = editingSessionId && ActivitySessions.sessionsOf(activity).find(item => item.id === entryKey)
    editing = session ? {...activity,...session,status:'completed'} : activity
    const source = editing
    const selector = byId('recordActivity')
    selector.replaceChildren(new Option('＋ 新建事项',''))
    for (const matter of state.activities) {
      const status = {running:'进行中',paused:'已暂停',completed:'已完成'}[matter.status]
      const duplicateName = state.activities.some(other => other.id !== matter.id && other.name === matter.name)
      selector.append(new Option(`${matter.name} · ${status}${duplicateName ? ` · ${matter.id.slice(-6)}` : ''}`,matter.id))
    }
    selector.value = ''
    byId('recordMatterField').hidden = scope !== 'new'
    byId('editActivityName').readOnly = scope === 'session'
    byId('editActivityName').value = activity?.name || ''
    updateMatterSelection()
    byId('editMatterStatusField').hidden = scope !== 'matter'
    byId('editMatterStatus').value = activity?.status || 'completed'
    byId('editProgressField').hidden = scope === 'matter'
    byId('editSessionProgress').value = session?.progress || ''
    byId('recordDateField').hidden = scope === 'matter'
    byId('editorTimes').hidden = scope === 'matter'
    byId('editSessionProgress').placeholder = scope === 'new' ? '本次进展（可选）' : '本次进展'
    byId('editActivityDate').disabled = scope === 'matter'
    for (const id of ['editActivityStart','editActivityEnd']) {
      const input = byId(id)
      input.closest('.editor-field').hidden = scope === 'matter'
      input.disabled = scope === 'matter'
      input.pattern = '[0-9]{2}:[0-9]{2}(:[0-9]{2})?'
    }
    document.querySelectorAll('.session-time-actions').forEach(element => { element.hidden = scope === 'matter' })
    byId('editActivityDuration').parentElement.hidden = scope === 'matter'
    const now = Date.now()
    let start = +new Date(`${dateInput.value}T09:00:00`)
    let end = start+3600000
    if (dateInput.value === localDate() && end > now) {
      end = now
      start = Math.max(dayBounds(dateInput.value)[0],now-3600000)
    }
    start = gap?.startTime || source?.startTime || start
    end = gap?.endTime || source?.endTime || end
    byId('editorTitle').textContent = scope === 'matter' ? '编辑事项' : scope === 'session' ? `编辑本次 · ${activity.name}` : '补录'
    byId('editorTitle').title = byId('editorTitle').textContent
    byId('editActivityDate').value = localDate(new Date(start))
    editorEndDayOffset = Math.round((Date.parse(`${localDate(new Date(end))}T00:00:00Z`)-Date.parse(`${byId('editActivityDate').value}T00:00:00Z`))/86400000)
    editorEndDayExplicit = Boolean(source || gap)
    byId('editActivityStart').value = localInput(start).split('T')[1]
    byId('editActivityEnd').value = activity && !source.endTime ? '' : localInput(end).split('T')[1]
    byId('editActivityStart').dataset.originalTime = source?.startTime || gap?.startTime || ''
    byId('editActivityEnd').dataset.originalTime = source?.endTime || gap?.endTime || ''
    byId('editActivityEnd').required = scope !== 'matter'
    updateEndDay()
    byId('editorError').textContent = ''
    updateEditorDuration()
    editor.showModal()
    byId('editorBody').scrollTop = 0
  }

  document.querySelectorAll('[data-page]').forEach(button => button.addEventListener('click', () => selectPage(button.dataset.page)))
  window.addEventListener('start-todo-activity', async event => {
    const { id, name } = event.detail
    const linked = state.activities.filter(activity => activity.todoId === id && activity.status !== 'completed')
      .sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))
    const activity = linked.find(activity => activity.status === 'running') || linked[0]
    if (activity) {
      if (activity.status === 'running' || await command('resume', {id:activity.id})) selectPage('current')
      return
    }
    if (await command('start', { name, todoId: id })) selectPage('current')
  })
  byId('startActivityForm').addEventListener('submit', async event => {
    event.preventDefault()
    const name = byId('activityName').value.trim()
    if (name && await command('start', {name})) { byId('activityName').value = ''; byId('activityName').focus() }
  })
  byId('pauseActivity').addEventListener('click', () => {
    const activity = selected()
    if (activity?.status === 'running') command('pause')
    else if (activity?.status === 'paused') command('resume', {id:activity.id})
  })
  byId('endActivity').addEventListener('click', () => { if (selected()) command('end', {id:selected().id}) })
  byId('restartActivity').addEventListener('click', () => {
    const activity = selected()
    if (activity?.status === 'completed') command('matter-edit', {id:activity.id,name:activity.name,status:'running'})
  })
  byId('backgroundToggle').addEventListener('click', () => {
    const expanded = byId('backgroundToggle').getAttribute('aria-expanded') === 'true'
    byId('backgroundToggle').setAttribute('aria-expanded', String(!expanded))
    byId('backgroundToggle').title = expanded ? '展开' : '收起'
    byId('pausedActivityList').hidden = expanded
    byId('backgroundActivities').classList.toggle('collapsed', expanded)
  })
  byId('quitApp').addEventListener('click', () => api.quit())
  byId('openToday').addEventListener('click', () => { dateInput.value = localDate(); showOverview(); selectPage('timeline') })
  dateInput.addEventListener('change', () => { closeMenu(false); renderTimeline() })
  function moveDay(offset) {
    closeMenu(false)
    const date = new Date(`${dateInput.value || localDate()}T12:00:00`)
    date.setDate(date.getDate() + offset)
    dateInput.value = localDate(date)
    deleteId = null
    renderTimeline()
  }
  byId('previousDay').addEventListener('click', () => moveDay(-1))
  byId('nextDay').addEventListener('click', () => moveDay(1))
  byId('todayButton').addEventListener('click', () => { closeMenu(false); dateInput.value = localDate(); renderTimeline() })

  function closeLegend() {
    byId('heatLegend').hidden = true
    byId('toggleLegend').setAttribute('aria-expanded','false')
  }
  byId('toggleLegend').addEventListener('click',() => {
    const open = byId('heatLegend').hidden
    byId('heatLegend').hidden = !open
    byId('toggleLegend').setAttribute('aria-expanded',String(open))
  })
  document.addEventListener('pointerdown',event => {
    if (!byId('heatLegend').contains(event.target) && !byId('toggleLegend').contains(event.target)) closeLegend()
  })
  document.addEventListener('keydown',event => {
    if (event.key === 'Escape' && !byId('heatLegend').hidden) {
      closeLegend()
      byId('toggleLegend').focus()
      event.preventDefault()
    }
  })

  byId('addActivity').addEventListener('click', () => openEditor())
  for (const id of ['closeEditor', 'cancelEditor']) byId(id).addEventListener('click', () => editor.close())
  editor.addEventListener('close', () => {
    closeTimePicker()
    const focused = document.activeElement
    if (detailActivityId) {
      (editorScope === 'matter' ? byId('editMatter') : [...document.querySelectorAll('.detail-session')].find(row=>row.dataset.sessionId===editingSessionId)?.querySelector('.session-edit'))?.focus()
    } else if (page === 'timeline' && (focused === document.body || editor.contains(focused))) {
      (findMenuTrigger(editorReturnActivityId, editorReturnEntryKey) || byId('addActivity')).focus()
    }
  })
  byId('activityEditorForm').addEventListener('input',updateEditorDuration)
  byId('recordActivity').addEventListener('change',updateMatterSelection)
  byId('editActivityDate').addEventListener('change',() => { updateEndDay(); updateEditorDuration() })
  byId('endDayToggle').addEventListener('click',() => {
    editorEndDayOffset = editorEndDayOffset === 0 ? 1 : 0
    editorEndDayExplicit = true
    updateEndDay()
    updateEditorDuration()
  })
  for (const id of ['editActivityStart','editActivityEnd']) byId(id).addEventListener('change',() => {
    updateEndDay()
    updateEditorDuration()
  })
  byId('activityEditorForm').addEventListener('submit', async event => {
    event.preventDefault()
    try {
      if (editorScope === 'matter') {
        if (await command('matter-edit',{id:editorReturnActivityId,name:byId('editActivityName').value,status:byId('editMatterStatus').value})) editor.close()
        return
      }
      const payload = { id: editing?.id, name: byId('editActivityName').value,
        startTime: inputTime(byId('editActivityStart')), endTime: byId('editActivityEnd').value ? inputTime(byId('editActivityEnd')) : null,
        segments: editorSegments() }
      if (editorScope === 'session') {
        if (await command('session-edit',{...payload,activityId:editorReturnActivityId,sessionId:editingSessionId,progress:byId('editSessionProgress').value})) editor.close()
      } else if (await command('session-add',{...payload,activityId:byId('recordActivity').value || null,progress:byId('editSessionProgress').value})) editor.close()
    } catch {
      byId('editorError').textContent = '请填写有效的日期与开始、结束时间'
      byId('editorError').scrollIntoView({ block: 'nearest' })
    }
  })

  const unsubscribe = api.onActivities(setState)
  api.getActivities().then(setState).catch(error => message(`活动读取失败：${error.message}`))
  const refreshTimer = setInterval(() => {
    if (document.hidden) return
    const activity = current()
    if (activity && page === 'current') {
      byId('currentTimer').textContent = clock(duration(activity))
      fitCurrentTimer()
    }
    if (page === 'timeline' && dateInput.value) {
      if (timelineRenderDay !== localDate()) { renderTimeline(); return }
      const bounds = dayBounds(dateInput.value)
      updateDayTotal(bounds, Date.now())
      const entry = activity && timelineEntries([activity], dateInput.value).find(entry => entry.status === 'running')
      const detail = entry && [...document.querySelectorAll('.timeline-item')].find(row => row.dataset.entryKey === entry.entryKey)?.querySelector('.timeline-duration')
      applyFocus()
      updateGaps(Date.now())
      if (detailActivityId && activity?.id === detailActivityId) {
        byId('matterTotal').textContent = humanDuration(duration(activity))
        for (const row of byId('matterSessions').querySelectorAll('.detail-session')) {
          const visit = timelineEntries([activity],row.dataset.day).find(item=>item.sessionId===row.dataset.sessionId)
          if (visit?.status === 'running') {
            row.querySelector('.detail-session-meta').textContent = `${timeLabel(visit.startTime,row.dataset.day)}–${visit.endTime ? timeLabel(visit.endTime,row.dataset.day) : '现在'} · ${humanDuration(duration(visit,Date.now(),dayBounds(row.dataset.day)))}`
            paintStrip(row,activity,duration(visit))
          }
        }
      }
      if (detail) {
        detail.textContent = humanDuration(duration(entry, Date.now(), bounds))
        paintStrip(detail.closest('.timeline-item'),entry,duration(entry))
      }
    }
  }, 1000)

  window.addEventListener("unload", () => { clearInterval(refreshTimer); unsubscribe() }, {once:true})
}
