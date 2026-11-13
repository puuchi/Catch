const fs=require('node:fs'),path=require('node:path')
const {TodosService}=require('../src/application/todos/service')
const {localDay}=require('../src/domain/todos')
let state={todos:[],archive:[]},now=Date.parse('2026-11-13T15:30:00Z'),step=0
const repository={load:()=>structuredClone(state),save:todos=>{state.todos=structuredClone(todos)},archive:(_before,after)=>{state=structuredClone(after)},recover:()=>{}}
const service=new TodosService(repository,{now:()=>now,newId:()=>`todo-${step}`})
const steps=[]
function run(type,payload={},advance=60_000){
  step++;now+=advance;let rejected=false
  try{service.command(type,payload,service.revision)}catch{rejected=true}
  const {error,...expected}=service.snapshot()
  steps.push({command:{type,...(type==='archive'?{}:{payload})},now,localDay:localDay(now),newId:`todo-${step}`,rejected,expected})
}
const fields={text:'  整理材料  ',notes:'第一行\n<script>文字</script>',urgent:false,deadline:null}
run('create',fields)
run('create',{...fields,text:'读一篇文章',urgent:true,deadline:'2026-11-15'})
run('edit',{...fields,id:'todo-1',text:'整理资料',notes:'新备注',deadline:'2026-11-30'})
run('complete',{id:'todo-1',done:true})
run('trash',{id:'todo-1'})
run('edit',{...fields,id:'todo-1'}) // reject
run('restore',{id:'todo-1'})
run('archive')
run('complete',{id:'todo-2',done:true},24*3600_000)
run('complete',{id:'todo-2',done:false})
run('delete',{id:'todo-2'}) // reject
run('trash',{id:'todo-2'})
run('delete',{id:'todo-2'})
run('create',{...fields,text:'',deadline:'2026-11-31'}) // reject
run('create',{...fields,text:'😀'.repeat(41)}) // JS measures UTF-16
run('create',{...fields,text:'有截止日期',deadline:'2026-11-31'}) // reject
run('create',{...fields,text:'归档测试'})
run('complete',{id:'todo-17',done:true})
run('archive')
const file=path.join(__dirname,'../crates/catch-core/tests/fixtures/todos-parity.json')
fs.writeFileSync(file,JSON.stringify({steps})+'\n')
console.log(`Wrote ${steps.length} todo steps`)
