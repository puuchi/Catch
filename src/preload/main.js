const electron=require('electron')
const invoke=(channel,...args)=>electron.ipcRenderer.invoke(channel,...args)
const subscribe=(topic,listener)=>{
  const handler=(_event,value)=>listener(value)
  electron.ipcRenderer.on(topic,handler)
  return ()=>electron.ipcRenderer.removeListener(topic,handler)
}
let activityRevision=0,todoRevision=0
const rememberActivity=state=>{activityRevision=Math.max(activityRevision,state.revision || 0);return state}
const rememberTodos=state=>{todoRevision=Math.max(todoRevision,state.revision || 0);return state}
const todos=()=>invoke('catch:read','todos').then(rememberTodos)
const todoCommand=command=>invoke('catch:command','todo',{...command,expectedRevision:command.expectedRevision ?? todoRevision}).then(result=>{if(result.state)rememberTodos(result.state);return result})
const control=(type,value)=>invoke('catch:window',type,value).catch(error=>console.error('[window]',error.message))
electron.contextBridge.exposeInMainWorld('api',{
  getTodos:todos,todoCommand,onTodos:fn=>subscribe('catch:todos',state=>fn(rememberTodos(state))),
  loadTodoTrash:()=>todos().then(state=>state.todos.filter(todo=>todo.deletedAt)),
  todoTrashAction:async(type,id)=>{await todos();return todoCommand({type,payload:{id},requestId:globalThis.crypto.randomUUID()})},
  getActivities:()=>invoke('catch:read','activity').then(rememberActivity),
  onActivities:fn=>subscribe('catch:activity',state=>fn(rememberActivity(state))),
  activityCommand:command=>invoke('catch:command','activity',{...command,expectedRevision:command.expectedRevision ?? activityRevision}).then(result=>{if(result.state)rememberActivity(result.state);return result}),
  getTransparency:()=>invoke('catch:read','appearance'),saveTransparency:value=>invoke('catch:appearance',value),
  minimize:()=>control('minimize'),close:()=>control('collapse'),setPin:value=>control('pin',value),quit:()=>control('quit')
})
