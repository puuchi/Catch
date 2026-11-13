const handlers={
  start:(s,p)=>s.start(p.name,p.todoId),pause:s=>s.pause(),resume:(s,p)=>s.resume(p.id),end:(s,p)=>s.end(p.id),
  edit:(s,p)=>s.edit(p),remove:(s,p)=>s.remove(p.id),
  'next-session':(s,p)=>s.nextSession(p.activityId,p.sessionId,p.progress),progress:(s,p)=>s.setProgress(p.activityId,p.sessionId,p.progress),
  'matter-edit':(s,p)=>s.updateMatter(p.id,p),'session-edit':(s,p)=>s.editSession(p),'session-add':(s,p)=>s.addSession(p),
  'session-delete':(s,p)=>s.deleteSession(p.activityId,p.sessionId),'trash-restore':(s,p)=>s.restoreTrash(p.id),'trash-delete':(s,p)=>s.deleteTrash(p.id)
}
function executeTracking(service,todos,command) {
  if(service.needsFlush)throw new Error('暂停状态尚未保存，请检查磁盘后重启应用')
  if(command.expectedRevision!==(service.data.revision || 0))throw Object.assign(new Error('投入记录已变化，请核对后重试'),{code:'CONFLICT'})
  if(command.type==='start' && command.payload.todoId) {
    const todo=todos.snapshot().todos.find(t=>t.id===command.payload.todoId && !t.done && !t.deletedAt)
    if(!todo)throw new Error('待办已完成或已删除')
    command={...command,payload:{...command.payload,name:todo.text}}
  }
  handlers[command.type](service,command.payload)
  return service.snapshot()
}
module.exports={executeTracking}
