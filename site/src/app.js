const {icon}=require('./icons')
const {stages:cropStages}=require('../../frontend/public/assets/plants/SOURCE.json')

const byId=id=>document.getElementById(id)
const frame=byId('catch-window')
const scene=byId('demo-stage')
const floating=byId('sprout-button')
const plant=byId('demo-floating-plant')
const simulateGrowth=byId('simulate-growth')
const simulateRepeat=byId('simulate-repeat')
const origin=window.location.origin

let ready=false, pending, pointer, moved=false
const send=(type,payload)=>frame.contentWindow.postMessage({catchDemo:type,payload},origin)

for(const element of document.querySelectorAll('[data-icon]'))element.append(icon(element.dataset.icon))

function collapse() {
  frame.hidden=true
  floating.hidden=false
  byId('demo-hint').textContent='单击植物展开，也可以拖动它'
}

function expand() {
  frame.hidden=false
  floating.hidden=true
  byId('demo-hint').textContent='点一下播放键，开始第一段投入'
}

function showPlant({plant:species,stage}) {
  const sprout=species==='sprout'
  const column=sprout?[0,8,17,26,35][stage]:cropStages[species].columns[stage]
  const row=sprout?0:cropStages[species].row
  const size=44
  Object.assign(plant.style,{
    backgroundImage:`url('${sprout?'assets/sprout/growing-plant-sprite.png':'assets/plants/Crop_Spritesheet.png'}')`,
    backgroundSize:`${size*(sprout?61:12)}px ${size*(sprout?1:10)}px`,
    backgroundPosition:`-${size*column}px -${size*row}px`,
    imageRendering:sprout?'auto':'pixelated',
  })
}

function show(type,payload) {
  expand()
  byId('experience').scrollIntoView({block:'center'})
  if(ready)send(type,payload)
  else pending={type,payload}
}

window.addEventListener('message',event=>{
  if(event.origin!==origin||event.source!==frame.contentWindow)return
  const {catchDemo,payload}=event.data || {}
  if(catchDemo==='ready') {
    ready=true
    if(pending) {send(pending.type,pending.payload);pending=null}
  }
  if(catchDemo==='collapse')collapse()
  if(catchDemo==='activity') {
    showPlant(payload.growth)
    byId('sprout-status').classList.toggle('running',payload.running)
    simulateGrowth.disabled=!payload.growth.selected||payload.growth.stage===4
    simulateRepeat.disabled=!payload.repeatDue
  }
})

byId('reset-demo').addEventListener('click',()=>{
  ready=false
  pending=null
  simulateGrowth.disabled=simulateRepeat.disabled=true
  floating.style.left=floating.style.top=''
  expand()
  frame.contentWindow.location.reload()
  byId('demo-announcement').textContent='已恢复初始示例'
})

byId('try-sprout').addEventListener('click',()=>{
  byId('experience').scrollIntoView({block:'center'})
  collapse()
  floating.focus({preventScroll:true})
})
for(const action of document.querySelectorAll('[data-demo-view]')) {
  action.addEventListener('click',()=>show('page',action.dataset.demoView))
}
document.querySelector('[data-demo-growth]').addEventListener('click',()=>show('growth'))
simulateGrowth.addEventListener('click',()=>send('advance-growth'))
simulateRepeat.addEventListener('click',()=>send('advance-repeat'))

floating.addEventListener('pointerdown',event=>{
  if(event.button!==0)return
  moved=false
  pointer={id:event.pointerId,x:event.clientX,y:event.clientY,left:floating.offsetLeft,top:floating.offsetTop}
  floating.setPointerCapture(event.pointerId)
})
floating.addEventListener('pointermove',event=>{
  if(pointer?.id!==event.pointerId)return
  const x=event.clientX-pointer.x,y=event.clientY-pointer.y
  if(!moved&&Math.hypot(x,y)<4)return
  moved=true
  floating.style.left=Math.max(12,Math.min(pointer.left+x,scene.clientWidth-floating.offsetWidth-12))+'px'
  floating.style.top=Math.max(12,Math.min(pointer.top+y,scene.clientHeight-floating.offsetHeight-40))+'px'
})
for(const type of ['pointerup','pointercancel','lostpointercapture'])floating.addEventListener(type,()=>{pointer=null})
floating.addEventListener('click',event=>{
  if(moved&&event.detail>0)return
  expand()
  frame.focus()
})
