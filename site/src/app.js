const {DemoState,duration,clock,statusText}=require('./demo-state')
const {element,button,compact,timeLabel}=require('./dom')
const {mountValidation,clearFormErrors}=require('../../src/ui/shared/form-validation')
const {icon}=require('./icons')
const mountSprout=require('../../src/ui/sprout/animation')
const byId=id=>document.getElementById(id)
const state=new DemoState()
const stage=byId('demo-stage')
const appWindow=byId('catch-window')
const sproutButton=byId('sprout-button')
const visibilityListeners=new Set()
let collapsed=false
let stageVisible=true
let progressSession=null
let gesture=null
let appPosition=null
let sproutPosition=null

mountValidation()
const timeline=require('./timeline')(state,announce)
for(const element of document.querySelectorAll('[data-icon]'))element.append(icon(element.dataset.icon))

const sprout=mountSprout({
  onBubbleVisibility:fn=>{visibilityListeners.add(fn);return ()=>visibilityListeners.delete(fn)},
  getBubbleVisibility:async()=>collapsed && stageVisible
})

function announce(message) {
  byId('demo-announcement').textContent=message
  byId('app-footer-state').textContent=message
}

function renderTodos() {
  const list=byId('demo-todos');list.replaceChildren()
  for(const todo of state.list) {
    const row=element('li',`${todo.urgent?'urgent ':''}${todo.done?'done':''}`)
    row.dataset.todoId=todo.id
    const check=button('check',`${todo.done?'取消完成':'完成'}：${todo.text}`,'todo-check')
    check.setAttribute('aria-pressed',String(todo.done))
    if(!todo.done)check.replaceChildren()
    check.addEventListener('click',()=>{state.toggleTodo(todo.id);render();announce(todo.done?'待办已完成':'已恢复为未完成')})
    const copy=element('div','todo-copy'),title=element('div','todo-title',todo.text)
    copy.append(title)
    if(todo.notes)copy.append(element('p','',todo.notes))
    row.append(check,copy)
    if(!todo.done) {
      const play=button('play','开始计时：'+todo.text,'icon-button todo-play')
      play.addEventListener('click',()=>{state.startTodo(todo.id);render();announce('开始投入，试试留下本次进展');byId('progress-input').focus({preventScroll:true})})
      row.append(play)
    }
    list.append(row)
  }
  byId('todo-count').textContent=`${state.todos.filter(todo=>!todo.done).length} 项未完成`
}

function renderCurrent() {
  const selected=state.selected,running=selected?.status==='running',completed=selected?.status==='completed'
  byId('current-name').textContent=selected?.name || '从眼前的一件事开始'
  byId('current-state').textContent=statusText(selected)
  byId('current-state').classList.toggle('completed',completed)
  byId('current-state').classList.toggle('running',running)
  const session=selected?.sessions.at(-1)
  byId('current-started').textContent=running?`${timeLabel(session.startTime)} 开始`:selected?`${selected.sessions.length} 次投入`:''
  byId('current-started').hidden=completed || !selected
  byId('demo-timer').textContent=selected?clock(duration(selected)):clock(0)
  const toggle=byId('toggle-timer');toggle.replaceChildren(icon(running?'pause':'play'))
  toggle.setAttribute('aria-label',running?'暂停计时':'继续计时');toggle.title=running?'暂停':'继续'
  toggle.disabled=!selected || completed
  byId('end-timer').disabled=!selected || completed
  toggle.hidden=!selected || completed
  byId('end-timer').hidden=!selected || completed
  byId('completed-summary').hidden=!completed
  byId('current-visits').textContent=selected?`累计 ${selected.sessions.length} 次投入`:''
  byId('next-step').hidden=!selected || completed
  byId('next-step').disabled=!running
  byId('save-progress').disabled=!selected
  byId('progress-input').disabled=!selected
  if(progressSession!==session?.id) {
    byId('progress-input').value=session?.progress || ''
    byId('progress-feedback').textContent=completed?'结束计时不会自动完成待办。':'Enter 保存进展，右侧箭头进入下一步。'
    progressSession=session?.id
  }
  const paused=byId('paused-matters');paused.replaceChildren()
  const others=state.tracking.paused().filter(item=>item.id!==selected?.id)
  byId('background-area').hidden=!others.length
  byId('background-count').textContent=`${others.length} 件事情，稍后继续`
  for(const matter of others) {
    const row=element('div','paused-item'),color=element('span','paused-color')
    color.style.backgroundColor=matter.color
    const copy=element('div','paused-copy');copy.append(element('strong','',matter.name),element('span','',compact(duration(matter))))
    const resume=button('play','继续：'+matter.name)
    resume.addEventListener('click',()=>{state.resume(matter.id);render();announce('已继续投入')})
    const end=button('stop','结束：'+matter.name)
    end.addEventListener('click',()=>{state.end(matter.id);render();announce('已结束后台事项')})
    row.append(color,copy,resume,end);paused.append(row)
  }
}

function renderTimeline() {timeline.render()}

function syncSprout() {
  const running=state.current
  sprout.update({status:running?'running':state.tracking.paused().length?'paused':'idle',activityId:running?.id})
  byId('sprout-status').classList.toggle('running',!!running)
  for(const listener of visibilityListeners)listener(collapsed && stageVisible)
}

function render() {
  for(const tab of document.querySelectorAll('[data-tab]')) {
    const active=tab.dataset.tab===state.page
    tab.setAttribute('aria-selected',String(active));tab.tabIndex=active?0:-1
    byId('panel-'+tab.dataset.tab).hidden=!active
  }
  byId('tab-status').classList.toggle('running',!!state.current)
  renderTodos();renderCurrent();renderTimeline();syncSprout()
  byId('demo-hint').textContent=collapsed?'单击绿芽展开，也可以把它拖到顺手的位置':state.page==='todos'?'↑ 点一下播放键，开始你的第一段投入':state.page==='current'?'试试暂停、记录进展，或点右上角收起小窗':'点击一段投入，聚焦同一件事'
}

function selectPage(page,focus=false) {
  timeline.closeLegend()
  state.page=page
  render()
  if(focus)byId('tab-'+page).focus({preventScroll:true})
}

for(const tab of document.querySelectorAll('[data-tab]')) {
  tab.addEventListener('click',()=>selectPage(tab.dataset.tab))
  tab.addEventListener('keydown',event=>{
    const pages=['todos','current','timeline'],index=pages.indexOf(state.page)
    if(['ArrowRight','ArrowLeft','Home','End'].includes(event.key)) {
      event.preventDefault()
      selectPage(event.key==='Home'?'todos':event.key==='End'?'timeline':pages[(index+(event.key==='ArrowRight'?1:2))%3],true)
    }
  })
}

byId('add-todo-form').addEventListener('submit',event=>{
  event.preventDefault()
  try{state.addTodo(byId('todo-input').value);byId('todo-input').value='';renderTodos();announce('记下了，试试点击旁边的播放键')}
  catch(error){announce(error.message)}
})
byId('start-matter-form').addEventListener('submit',event=>{
  event.preventDefault()
  try{state.startMatter(byId('matter-input').value);byId('matter-input').value='';render();announce('已开始新的投入')}
  catch(error){announce(error.message)}
})
byId('toggle-timer').addEventListener('click',()=>{state.toggleTimer();render();announce(state.current?'正在投入':'已暂停，回来再继续')})
byId('end-timer').addEventListener('click',()=>{state.end();render();byId('progress-feedback').textContent='结束计时不会自动完成待办。';announce('已结束本事项，待办状态保留')})
byId('restart-matter').addEventListener('click',()=>{
  try{state.restart();render();announce('已重新开始，累计时长和历史进展已保留')}
  catch(error){announce(error.message)}
})
byId('progress-form').addEventListener('submit',event=>{
  event.preventDefault()
  try{state.progress(byId('progress-input').value);byId('progress-feedback').textContent='已留下这次的进展。';renderTimeline();announce('本次进展已更新到示例时间轴')}
  catch(error){announce(error.message)}
})
byId('progress-input').addEventListener('input',()=>{byId('progress-feedback').textContent='Enter 保存进展，右侧箭头进入下一步。'})
byId('next-step').addEventListener('click',()=>{
  try{state.next(byId('progress-input').value);render();byId('progress-input').focus({preventScroll:true});announce('新的一步开始了，之前的进展保留在时间轴')}
  catch(error){announce(error.message)}
})
byId('background-toggle').addEventListener('click',()=>{
  const expanded=byId('background-toggle').getAttribute('aria-expanded')==='true'
  byId('background-toggle').setAttribute('aria-expanded',String(!expanded))
  byId('background-toggle').setAttribute('aria-label',expanded?'展开暂停事项':'收起暂停事项')
  byId('paused-matters').hidden=expanded
})

function limitPosition(target,position) {
  const padding=13
  return {x:Math.max(padding,Math.min(position.x,stage.clientWidth-target.offsetWidth-padding)),y:Math.max(15,Math.min(position.y,stage.clientHeight-target.offsetHeight-39))}
}
function place(target,position) {
  const point=limitPosition(target,position)
  target.style.left=point.x+'px';target.style.top=point.y+'px'
  return point
}
function collapse() {
  if(collapsed)return
  sproutButton.hidden=false
  const x=appWindow.offsetLeft+(appWindow.offsetWidth-sproutButton.offsetWidth)/2,y=appWindow.offsetTop+15
  collapsed=true;appWindow.hidden=true;sproutButton.hidden=false
  sproutPosition=place(sproutButton,{x,y})
  render();announce('收起为绿芽，计时仍按当前状态继续')
}
function expand() {
  if(!collapsed)return
  collapsed=false;appWindow.hidden=false;sproutButton.hidden=true
  appPosition=place(appWindow,{x:sproutPosition.x,y:sproutPosition.y})
  render();byId('collapse-window').focus({preventScroll:true});announce('回到刚才的事情')
}
byId('collapse-window').addEventListener('click',collapse)
byId('try-sprout').addEventListener('click',()=>{
  byId('experience').scrollIntoView({behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth',block:'center'})
  if(!collapsed)collapse()
  else announce('单击演示区域的小绿芽即可展开')
  sproutButton.focus({preventScroll:true})
})
for(const action of document.querySelectorAll('[data-demo-view]'))action.addEventListener('click',()=>{
  if(collapsed)expand()
  selectPage(action.dataset.demoView)
  byId('experience').scrollIntoView({behavior:matchMedia('(prefers-reduced-motion: reduce)').matches?'instant':'smooth',block:'center'})
  byId('tab-'+state.page).focus({preventScroll:true})
})

function draggable(handle,target,isSprout=false) {
  handle.addEventListener('pointerdown',event=>{
    if(event.button!==0 || event.target.closest('button') && !isSprout || gesture)return
    gesture={id:event.pointerId,x:event.clientX,y:event.clientY,left:target.offsetLeft,top:target.offsetTop,moved:false}
    handle.setPointerCapture(event.pointerId)
  })
  handle.addEventListener('pointermove',event=>{
    if(!gesture || gesture.id!==event.pointerId)return
    const dx=event.clientX-gesture.x,dy=event.clientY-gesture.y
    if(Math.hypot(dx,dy)>=4)gesture.moved=true
    if(!gesture.moved)return
    target.classList.add('dragging')
    const point=place(target,{x:gesture.left+dx,y:gesture.top+dy})
    if(isSprout)sproutPosition=point
    else appPosition=point
  })
  handle.addEventListener('pointerup',event=>{
    if(!gesture || gesture.id!==event.pointerId)return
    const open=isSprout&&!gesture.moved
    gesture=null;target.classList.remove('dragging');handle.releasePointerCapture(event.pointerId)
    if(open)expand()
  })
  for(const name of ['pointercancel','lostpointercapture'])handle.addEventListener(name,()=>{gesture=null;target.classList.remove('dragging')})
}
draggable(byId('window-handle'),appWindow)
draggable(sproutButton,sproutButton,true)
sproutButton.addEventListener('click',event=>{if(event.detail===0)expand()})

byId('reset-demo').addEventListener('click',()=>{
  timeline.closeLegend()
  for(const form of document.forms)clearFormErrors(form)
  byId('paused-matters').hidden=false
  byId('background-toggle').setAttribute('aria-expanded','true')
  byId('background-toggle').setAttribute('aria-label','收起暂停事项')
  state.reset();collapsed=false;progressSession=null;appPosition=null;sproutPosition=null
  appWindow.hidden=false;sproutButton.hidden=true
  appWindow.style.left='';appWindow.style.top=''
  byId('todo-input').value='';byId('matter-input').value=''
  render();announce('已恢复初始示例')
})
new ResizeObserver(()=>{
  if(collapsed&&sproutPosition)sproutPosition=place(sproutButton,sproutPosition)
  else if(appPosition)appPosition=place(appWindow,appPosition)
}).observe(stage)
if('IntersectionObserver' in window)new IntersectionObserver(entries=>{
  stageVisible=entries.some(entry=>entry.isIntersecting)
  syncSprout()
},{threshold:0}).observe(stage)

const refresh=setInterval(()=>{
  if(document.hidden || !stageVisible)return
  if(!collapsed && state.page==='current')byId('demo-timer').textContent=state.selected?clock(duration(state.selected)):clock(0)
  if(!collapsed && state.page==='timeline' && state.current)renderTimeline()
},1000)
window.addEventListener('pagehide',()=>clearInterval(refresh),{once:true})
render()
