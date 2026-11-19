// Generates synthetic compatibility evidence using the existing shipping rules.
// No user profile is opened. Run after intentionally changing the specification,
// then review the fixture diff; Rust tests never regenerate their own oracle.
const fs = require('node:fs')
const path = require('node:path')
const { TrackingModel } = require('../src/domain/tracking/model')
const { validateDocument } = require('../src/domain/tracking/validation')
const base = Date.parse('2026-11-13T15:30:00.000Z')
const initial = () => ({version:2,sessionVersion:1,revision:0,checkpointAt:null,activities:[],trash:[]})
const scenarios = []
function scenario(name, build) {
  let now=base, step=0
  const model=new TrackingModel(initial(),{now:()=>now,newId:()=>`step-${step}`})
  const steps=[]
  function run(type,payload={},advance=60_000) {
    now+=advance; step++
    const before=model.snapshot()
    let error=null
    try {
      switch(type) {
        case 'start': model.start(payload.name,payload.todoId); break
        case 'pause': model.pause(payload.reason,payload.cutoff); break
        case 'resume': model.resume(payload.id); break
        case 'end': model.end(payload.id); break
        case 'edit': model.edit(payload); break
        case 'remove': model.remove(payload.id); break
        case 'add-session': model.addSession(payload); break
        case 'set-progress': model.setProgress(payload.activityId,payload.sessionId,payload.progress); break
        case 'next-session': model.nextSession(payload.activityId,payload.sessionId,payload.progress); break
        case 'update-matter': model.updateMatter(payload.id,payload); break
        case 'edit-session': model.editSession(payload); break
        case 'delete-session': model.deleteSession(payload.activityId,payload.sessionId); break
        case 'restore-trash': model.restoreTrash(payload.id); break
        case 'delete-trash': model.deleteTrash(payload.id); break
        case 'checkpoint': model.commit(()=>{}); break
        default: throw Error('Unknown fixture operation')
      }
      validateDocument(model.snapshot())
    } catch(cause) {error=cause.message}
    if(error && JSON.stringify(before)!==JSON.stringify(model.snapshot()))throw Error('Oracle mutation on rejection')
    steps.push({command:{type,...(type==='checkpoint'?{}:{payload})},now,newId:`step-${step}`,rejected:!!error,expected:model.snapshot()})
  }
  const iso=offset=>new Date(base+offset*60_000).toISOString()
  build(run,iso)
  scenarios.push({name,initial:initial(),steps})
}
scenario('switch, pause, progress, restart, and two kinds of recycle',(run)=>{
  run('start',{name:'阅读',todoId:'todo-1'}) // step-1
  run('pause',{})
  run('resume',{id:'step-1'},20*60_000)
  run('next-session',{activityId:'step-1',sessionId:'step-1:1',progress:'第一节读完'})
  run('set-progress',{activityId:'step-1',sessionId:'step-1:2',progress:'  第二节  '})
  run('start',{name:'整理资料'}) // step-6
  run('resume',{id:'step-1'})
  run('end',{})
  run('update-matter',{id:'step-1',name:'阅读笔记',status:'running'})
  run('pause',{})
  run('delete-session',{activityId:'step-1',sessionId:'step-1:1'}) // step-11
  run('restore-trash',{id:'step-11'})
  run('remove',{id:'step-1'}) // step-13
  run('restore-trash',{id:'step-13'})
  run('remove',{id:'step-6'}) // step-15
  run('delete-trash',{id:'step-15'})
  run('pause',{}) // no-op does not increment revision
})
scenario('manual intervals, zero durations, overlap and immutable rejection',(run,iso)=>{
  const interval=(a,b)=>({start:iso(a),end:iso(b)})
  run('add-session',{name:'补记',startTime:iso(-60),endTime:iso(-50),progress:'补记进展'})
  run('add-session',{activityId:'step-1',startTime:iso(-45),endTime:iso(-40)})
  run('edit',{id:'step-1',name:'不能抹掉多次记录',startTime:iso(-60),endTime:iso(-40),segments:[interval(-60,-40)]}) // reject
  run('edit-session',{activityId:'step-1',sessionId:'step-1:1',startTime:iso(-44),endTime:iso(-40),progress:'调整',segments:[interval(-44,-40)]})
  run('add-session',{name:'冲突',startTime:iso(-55),endTime:iso(-49)}) // reject
  run('edit',{name:'零时长',startTime:iso(-30),endTime:iso(-30),segments:[]})
  run('edit',{id:'step-6',name:'零时长改名',startTime:iso(-30),endTime:iso(-30),segments:[]})
  run('delete-session',{activityId:'step-1',sessionId:'step-1:0'}) // step-8
  run('restore-trash',{id:'step-8'})
  run('edit-session',{activityId:'step-1',sessionId:'step-1:0',startTime:iso(-60),endTime:iso(-50),progress:'独立时间段',segments:[interval(-60,-57),interval(-53,-50)]})
  run('edit',{name:'未来',startTime:iso(100),endTime:iso(101),segments:[interval(100,101)]}) // reject
})
scenario('restore refuses conflicts and retains deleted record',(run,iso)=>{
  const input={name:'旧记录',startTime:iso(-20),endTime:iso(-10),segments:[{start:iso(-20),end:iso(-10)}]}
  run('edit',input)
  run('remove',{id:'step-1'})
  run('edit',{...input,name:'占用同一时间'})
  run('restore-trash',{id:'step-2'}) // reject
  run('remove',{id:'step-3'})
  run('restore-trash',{id:'step-2'})
  run('delete-trash',{id:'step-5'})
})
scenario('clock rollback, cutoff, stale session and multiple paused choices',(run)=>{
  run('start',{name:'A'})
  run('pause',{})
  run('resume',{id:'step-1'},-60_000) // reject rollback
  run('resume',{id:'step-1'},120_000)
  run('next-session',{activityId:'step-1',sessionId:'step-1:0',progress:'旧草稿'}) // reject
  run('pause',{reason:'clock-gap',cutoff:base+4*60_000},3*60_000)
  run('start',{name:'B'})
  run('pause',{})
  run('resume',{}) // reject ambiguous
  run('resume',{id:'step-1'})
  run('resume',{id:'step-1'}) // no-op
  run('delete-session',{activityId:'step-1',sessionId:'step-1:2'}) // stops active first
  run('restore-trash',{id:'step-12'})
})
for(const order of ['session-first','matter-first'])scenario('deleted parent and session restore '+order,(run)=>{
  run('start',{name:'A'})
  run('pause',{})
  run('resume',{id:'step-1'})
  run('end',{})
  run('delete-session',{activityId:'step-1',sessionId:'step-1:0'})
  run('remove',{id:'step-1'})
  for(const id of order==='session-first'?['step-5','step-6']:['step-6','step-5'])run('restore-trash',{id})
})
scenario('historical visit editing and insertion preserve a running foreground',(run,iso)=>{
  run('start',{name:'A'})
  run('pause',{})
  run('resume',{id:'step-1'})
  run('add-session',{activityId:'step-1',startTime:iso(-10),endTime:iso(-5),progress:'更早的投入'})
  run('edit-session',{activityId:'step-1',sessionId:'step-1:2',startTime:iso(-9),endTime:iso(-5),progress:'调整',segments:[{start:iso(-9),end:iso(-5)}]})
  run('delete-session',{activityId:'step-1',sessionId:'step-1:0'})
  run('restore-trash',{id:'step-6'})
  run('end',{})
})
scenario('palette selection and foreground interleaving',(run)=>{
  for(let i=0;i<12;i++)run('start',{name:'事项 '+i})
  run('end',{id:'step-1'})
  run('remove',{id:'step-2'})
  run('start',{name:'新颜色'})
})
const destination=path.join(__dirname,'../crates/catch-core/tests/fixtures/tracking-parity.json')
fs.mkdirSync(path.dirname(destination),{recursive:true})
fs.writeFileSync(destination,JSON.stringify({source:'Electron TrackingModel at 38413e5d',scenarios})+'\n')
console.log(`Wrote ${scenarios.length} scenarios / ${scenarios.reduce((n,s)=>n+s.steps.length,0)} steps`)
