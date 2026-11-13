const {duration,localDate,dayBounds,durationOpacity}=require('./demo-state')
const {element,compact,timeLabel}=require('./dom')

module.exports=function mountTimeline(state,announce) {
  const byId=id=>document.getElementById(id)
  const date=byId('timeline-date')

  function render() {
    date.value=state.day
    const visits=state.visits(),bounds=dayBounds(state.day)
    byId('timeline-total').textContent=compact(state.dayTotal())
    byId('timeline-total-label').textContent=state.day===localDate(new Date(state.now()))?'今日已记录':'当日已记录'
    const list=byId('demo-timeline'),focusedEntry=document.activeElement?.dataset.entryKey
    list.replaceChildren()
    if(!visits.length)list.append(element('p','timeline-empty','这一天还没有记录。换个日期，看看其他示例记录。'))
    for(const visit of visits) {
      const milliseconds=duration(visit,state.now(),bounds)
      const entry=element('button','timeline-entry');entry.type='button';entry.dataset.matterId=visit.id
      entry.dataset.entryKey=visit.entryKey
      entry.classList.toggle('dimmed',!!state.timelineFocus&&state.timelineFocus!==visit.id)
      entry.setAttribute('aria-pressed',String(state.timelineFocus===visit.id))
      entry.setAttribute('aria-label',`${visit.name}，${compact(milliseconds)}，点击聚焦`)
      const times=element('span','timeline-times')
      times.append(element('span','',timeLabel(visit.startTime)),element('span','',visit.endTime?timeLabel(visit.endTime):'现在'))
      const line=element('span','timeline-line');line.style.backgroundColor=visit.color;line.style.opacity=durationOpacity(milliseconds)
      const copy=element('span','timeline-entry-copy'),heading=element('span','timeline-entry-title')
      heading.append(element('strong','',visit.name),element('span','timeline-duration',compact(milliseconds)))
      copy.append(heading,element('span','timeline-progress',visit.progress||(visit.endTime?'一次小小的投入':'正在投入…')))
      entry.append(times,line,copy)
      entry.addEventListener('click',()=>{
        state.timelineFocus=state.timelineFocus===visit.id?null:visit.id
        render();announce(state.timelineFocus?'已聚焦这个事项':'已显示全部投入')
      })
      list.append(entry)
    }
    const detail=byId('timeline-detail')
    byId('clear-focus').hidden=!state.timelineFocus;detail.hidden=!state.timelineFocus
    if(state.timelineFocus) {
      const matter=state.tracking.get(state.timelineFocus),focused=visits.filter(item=>item.id===state.timelineFocus)
      detail.replaceChildren(element('strong','',matter.name),element('p','',`当日 ${focused.length} 次投入 · ${compact(duration(matter,state.now(),bounds))}`))
      for(const visit of focused)if(visit.progress)detail.append(element('p','',visit.progress))
    }
    if(focusedEntry)[...list.children].find(item=>item.dataset.entryKey===focusedEntry)?.focus({preventScroll:true})
  }

  function closeLegend() {
    byId('timeline-legend').hidden=true
    byId('toggle-legend').setAttribute('aria-expanded','false')
  }
  function changeDay(action) {closeLegend();action();render()}
  byId('previous-day').addEventListener('click',()=>changeDay(()=>state.moveDay(-1)))
  byId('next-day').addEventListener('click',()=>changeDay(()=>state.moveDay(1)))
  byId('today-button').addEventListener('click',()=>changeDay(()=>state.selectDay(localDate(new Date(state.now())))))
  date.addEventListener('change',()=>{
    if(!date.value){date.value=state.day;return}
    changeDay(()=>state.selectDay(date.value))
  })
  byId('toggle-legend').addEventListener('click',()=>{
    const open=byId('timeline-legend').hidden
    byId('timeline-legend').hidden=!open
    byId('toggle-legend').setAttribute('aria-expanded',String(open))
  })
  document.addEventListener('pointerdown',event=>{
    if(!byId('timeline-legend').contains(event.target)&&!byId('toggle-legend').contains(event.target))closeLegend()
  })
  document.addEventListener('keydown',event=>{
    if(event.key==='Escape'&&!byId('timeline-legend').hidden){closeLegend();byId('toggle-legend').focus();event.preventDefault()}
  })
  byId('clear-focus').addEventListener('click',()=>{state.timelineFocus=null;render()})

  return {render,closeLegend}
}
