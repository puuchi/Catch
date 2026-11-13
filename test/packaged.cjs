// Exercises the real packaged executable through a loopback-only debug port.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict')
const {spawn}=require('node:child_process'),net=require('node:net')
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms))
async function freePort(){const server=net.createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const port=server.address().port;await new Promise(resolve=>server.close(resolve));return port}
async function run() {
  const temporaryRoot=fs.realpathSync(os.tmpdir())
  const directory=fs.mkdtempSync(path.join(temporaryRoot,'catch-packaged-')),port=await freePort()
  const executable=path.resolve(process.argv[2] || 'dist/win-unpacked/Catch.exe')
  const legacyTodo={id:'legacy-todo',text:'旧版示例待办',notes:'保留旧版备注',urgent:true,done:false,deadline:null}
  const startTime='2026-11-12T09:00:00.000Z',endTime='2026-11-12T09:12:00.000Z'
  fs.writeFileSync(path.join(directory,'todos.json'),JSON.stringify([legacyTodo]))
  fs.writeFileSync(path.join(directory,'activity-log.json'),JSON.stringify({version:1,checkpointAt:endTime,activities:[{
    id:'legacy-matter',name:'旧版示例投入',startTime,endTime,status:'completed',segments:[{start:startTime,end:endTime}],todoId:legacyTodo.id,
    createdAt:startTime,updatedAt:endTime,pauseReason:null
  }]}))
  const env={...process.env,CATCH_TEST_DATA:directory};delete env.ELECTRON_RUN_AS_NODE
  const child=spawn(executable,[`--remote-debugging-port=${port}`,'--remote-debugging-address=127.0.0.1'],{env,windowsHide:true,stdio:'ignore'})
  let socket
  try {
    let target
    for(let i=0;i<150;i++){
      try{target=(await(await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find(t=>t.type==='page'&&t.url.includes('index.html'));if(target)break}catch{}
      await sleep(100)
    }
    assert.ok(target,'packaged main page did not become available')
    socket=new WebSocket(target.webSocketDebuggerUrl)
    await new Promise((resolve,reject)=>{socket.onopen=resolve;socket.onerror=reject})
    let sequence=0;const pending=new Map()
    socket.onmessage=event=>{const reply=JSON.parse(event.data);if(reply.id){pending.get(reply.id)?.(reply);pending.delete(reply.id)}}
    const call=(method,params)=>new Promise(resolve=>{const id=++sequence;pending.set(id,resolve);socket.send(JSON.stringify({id,method,params}))})
    const evaluate=async expression=>{
      const reply=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true})
      if(reply.error || reply.result.exceptionDetails)throw new Error(JSON.stringify(reply))
      return reply.result.result.value
    }
    for(let i=0;i<100;i++){if(await evaluate('Boolean(window.api && document.getElementById("todoInput"))'))break;await sleep(50)}
    const result=await evaluate(`window.api.getTodos().then(s=>window.api.todoCommand({type:'create',payload:{text:'安装包验证',notes:'隔离数据',urgent:false,deadline:null},expectedRevision:s.revision,requestId:crypto.randomUUID()}))`)
    assert.equal(result.ok,true)
    const savedTodos=JSON.parse(fs.readFileSync(path.join(directory,'todos.json'),'utf8'))
    assert.ok(savedTodos.some(todo=>todo.text==='安装包验证'))
    assert.deepEqual(savedTodos.find(todo=>todo.id===legacyTodo.id),legacyTodo)
    const migrated=JSON.parse(fs.readFileSync(path.join(directory,'activity-log.json'),'utf8'))
    assert.equal(migrated.version,2)
    assert.equal(migrated.sessionVersion,1)
    assert.equal(migrated.activities[0].id,'legacy-matter')
    assert.equal(migrated.activities[0].todoId,legacyTodo.id)
    assert.equal(migrated.activities[0].sessions.length,1)
    assert.equal(require('../src/domain/tracking/time').duration(migrated.activities[0]),12*60000)
    assert.ok(fs.readdirSync(directory).some(name=>name.startsWith('activity-log.json.v1-backup-')))
    assert.equal(await evaluate("Boolean(document.getElementById('restartActivity') && document.getElementById('completedSummary'))"),true)
    assert.equal(await evaluate("getComputedStyle(document.getElementById('nextProgress')).cursor"),'default')
    await evaluate("document.querySelector('[data-page=timeline]').click()")
    const image=await call('Page.captureScreenshot',{format:'png'})
    fs.mkdirSync(path.resolve('artifacts/qa'),{recursive:true})
    fs.writeFileSync(path.resolve('artifacts/qa/packaged.png'),Buffer.from(image.result.data,'base64'))
    await evaluate('window.api.quit()')
    await Promise.race([new Promise(resolve=>child.once('exit',resolve)),sleep(5000)])
    assert.ok(child.exitCode!==null,'packaged app did not exit cleanly')
    console.log('PASS packaged executable: isolated launch, current UI, legacy migration/backup, preserved records, committed data and clean exit')
  } finally {
    socket?.close()
    if(child.exitCode===null){child.kill();await new Promise(resolve=>child.once('exit',resolve))}
    const resolved=path.resolve(directory)
    if(path.dirname(resolved)!==temporaryRoot || !path.basename(resolved).startsWith('catch-packaged-'))throw new Error('Unexpected packaged-test profile path')
    fs.rmSync(resolved,{recursive:true,force:true,maxRetries:8,retryDelay:200})
  }
}
run().catch(error=>{console.error(error.stack);process.exitCode=1})
