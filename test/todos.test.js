const {test}=require('node:test')
const assert=require('node:assert/strict')
const fs=require('node:fs'),path=require('node:path'),os=require('node:os')
const {TodosRepository}=require('../src/infrastructure/persistence/todos-repository')
const {TodosService}=require('../src/application/todos/service')
const {Commands}=require('../src/application/commands')
const {visibleTodos,history,deadlineLabel}=require('../src/domain/todos')
function fixture(t,options={}) {
  const directory=fs.mkdtempSync(path.join(os.tmpdir(),'catch-todos-'))
  t.after(()=>fs.rmSync(directory,{recursive:true,force:true}))
  const repo=new TodosRepository(directory,options)
  let id=0
  const service=new TodosService(repo,{now:()=>new Date('2026-11-12T12:00:00').getTime(),newId:()=>String(++id)})
  const run=(type,payload={})=>service.command(type,payload,service.revision)
  return {directory,repo,service,run}
}
const content={text:'记录想法',notes:'第一行\n<script>文字</script>',urgent:false,deadline:null}
test('todo lifecycle preserves notes, completion, recycle and archive across reload',t=>{
  const f=fixture(t);f.run('create',content);f.run('edit',{...content,id:'1',urgent:true})
  f.run('complete',{id:'1',done:true});f.run('trash',{id:'1'});f.run('restore',{id:'1'});f.run('archive')
  const data=new TodosRepository(f.directory).load()
  assert.equal(data.todos.length,0);assert.equal(data.archive[0].notes,content.notes)
  assert.equal(history(data.todos,data.archive).length,1)
})
test('write failure preserves committed records and stale edits are rejected',t=>{
  const f=fixture(t);f.run('create',content)
  const before=f.service.snapshot();f.repo.write=()=>{throw new Error('disk full')}
  assert.throws(()=>f.run('edit',{...content,id:'1',text:'丢弃'}),/保存/)
  assert.deepEqual(f.service.state.todos,before.todos)
  assert.throws(()=>f.service.command('edit',{...content,id:'1'},before.revision),/变化/)
})
for(const stage of ['journal','archive','todos','cleanup'])test('archive resumes after interruption at '+stage,t=>{
  const f=fixture(t);f.run('create',content);f.run('complete',{id:'1',done:true})
  const before=f.service.state,next={todos:[],archive:before.todos}
  f.repo.fault=step=>{if(step===stage)throw new Error('simulated process termination')}
  assert.throws(()=>f.repo.archive(before,next))
  const recovered=new TodosRepository(f.directory)
  assert.equal(recovered.load().todos.length,0);assert.equal(recovered.load().archive.length,1)
  recovered.recover();assert.equal(recovered.load().archive[0].notes,content.notes)
})
test('corrupt archive is preserved and does not become an empty successful history',t=>{
  const f=fixture(t);const file=path.join(f.directory,'archive.json');fs.writeFileSync(file,'{broken')
  assert.throws(()=>new TodosRepository(f.directory).load());assert.equal(fs.readFileSync(file,'utf8'),'{broken')
})
test('pending recovery refuses external mutations',t=>{
  const f=fixture(t);f.run('create',content);f.run('complete',{id:'1',done:true})
  f.repo.fault=()=>{throw new Error('interrupt')}
  assert.throws(()=>f.repo.archive(f.service.state,{todos:[],archive:f.service.state.todos}))
  fs.writeFileSync(path.join(f.directory,'todos.json'),'[]')
  fs.writeFileSync(path.join(f.directory,'archive.json'),JSON.stringify([{id:'external',text:'外部',done:true}]))
  assert.throws(()=>new TodosRepository(f.directory),/外部修改/)
})
test('query sorting is stable and deadline arithmetic ignores daylight saving offsets',()=>{
  const rows=[{id:'a',text:'a',done:false},{id:'b',text:'b',done:false,urgent:true},{id:'c',text:'c',done:false,deadline:'2026-11-12'}]
  assert.deepEqual(visibleTodos(rows,'2026-11-12').map(t=>t.id),['b','c','a'])
  assert.equal(deadlineLabel('2026-03-09','2026-03-08'),'3月9日 · 1天后')
})
test('serialized commands replay their original result and reject request ID collisions',async()=>{
  const queue=new Commands();let count=0
  const command={requestId:'same',type:'create'}
  const [a,b]=await Promise.all([queue.run(command,()=>++count),queue.run(command,()=>++count)])
  assert.equal(count,1);assert.equal(a,b)
  await assert.rejects(queue.run({...command,type:'delete'},()=>++count),/不一致/)
  assert.equal(await queue.run({requestId:'next'},()=>++count),2)
})
