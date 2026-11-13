const {duration}=require('../../domain/tracking/time')
const api=window.api
const sprout=require('./animation')(api)
const target=document.getElementById('bubble')
let currentState,revision=-1,gesture
function update(state) {
  if((state.revision||0)<revision)return
  revision=state.revision||0;currentState=state
  const running=state.activities.find(item=>item.status==='running')
  const paused=state.activities.filter(item=>item.status==='paused').length
  const status=running?'running':paused?'paused':'idle'
  sprout.update({status,activityId:running?.id})
  document.getElementById('activityDot').className=status
  target.title=running?`正在 · ${running.name} · ${Math.floor(duration(running)/60000)} 分钟`:paused?`已暂停 · ${paused} 个后台事项`:'Catch · 单击展开，右键关闭'
}
const unsubscribe=api.onActivities(update)
api.getActivities().then(update).catch(error=>console.error(error.message))
target.onmouseenter=()=>{if(currentState)update(currentState)}
// Authorized PR pointer gesture behavior, reconnected to the new desktop bridge.
target.onpointerdown=event=>{
  if(event.button!==0||gesture)return
  gesture={id:event.pointerId,x:event.screenX,y:event.screenY,offsetX:event.clientX,offsetY:event.clientY,moved:false}
  target.setPointerCapture(event.pointerId)
}
target.onpointermove=event=>{
  if(!gesture||gesture.id!==event.pointerId)return
  if(Math.hypot(event.screenX-gesture.x,event.screenY-gesture.y)>=4)gesture.moved=true
  if(gesture.moved)api.moveWindow(event.screenX-gesture.offsetX,event.screenY-gesture.offsetY)
}
target.onpointerup=event=>{
  if(!gesture||gesture.id!==event.pointerId)return
  const expand=!gesture.moved;gesture=null;target.releasePointerCapture(event.pointerId)
  if(expand)api.restoreMain()
}
target.onpointercancel=target.onlostpointercapture=()=>{gesture=null}
document.oncontextmenu=event=>{event.preventDefault();gesture=null;api.showContextMenu()}
window.addEventListener('unload',unsubscribe,{once:true})
