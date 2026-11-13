const {performance}=require('node:perf_hooks')
const fs=require('node:fs'),path=require('node:path'),os=require('node:os')
const {TodosRepository}=require('../src/infrastructure/persistence/todos-repository')
const {TodosService}=require('../src/application/todos/service')
const {visibleTodos}=require('../src/domain/todos')
const results=[]
for(const count of [100,1000,10000]) {
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'catch-benchmark-'))
  try {
    const rows=Array.from({length:count},(_,i)=>({id:String(i),text:`合成待办 ${i}`,notes:'测试数据',done:false,urgent:i%7===0,deadline:null}))
    fs.writeFileSync(path.join(dir,'todos.json'),JSON.stringify(rows))
    const start=performance.now(),service=new TodosService(new TodosRepository(dir),{now:Date.now,newId:()=>String(count)})
    const loaded=performance.now();visibleTodos(service.state.todos,'2026-11-13');const queried=performance.now()
    service.command('create',{text:'新增合成待办',notes:'',urgent:false,deadline:null},0)
    results.push({todos:count,loadMs:+(loaded-start).toFixed(2),queryMs:+(queried-loaded).toFixed(2),saveMs:+(performance.now()-queried).toFixed(2)})
  }finally{fs.rmSync(dir,{recursive:true,force:true})}
}
fs.mkdirSync(path.resolve(__dirname,'../artifacts'),{recursive:true})
fs.writeFileSync(path.resolve(__dirname,'../artifacts/benchmark.json'),JSON.stringify(results,null,2))
console.log(JSON.stringify(results,null,2))
