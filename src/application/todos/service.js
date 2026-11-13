// @ts-check
const {fields,localDay,history} = require('../../domain/todos')
/** @typedef {import('../../domain/todos').Todo} Todo */
/** @typedef {{load:()=>{todos:Todo[],archive:Todo[]},save:(todos:Todo[])=>void,archive:(before:any,after:any)=>void,recover:()=>void}} Repository */
class TodosService {
  /** @param {Repository} repository @param {{now:()=>number,newId:()=>string}} dependencies */
  constructor(repository,{now,newId}) {
    this.repository=repository; this.now=now; this.newId=newId
    this.state=repository.load(); this.revision=0; this.error=null
  }
  snapshot() { return {...structuredClone(this.state),revision:this.revision,error:this.error} }
  /** @param {string} type @param {any} payload @param {number} expectedRevision */
  command(type,payload,expectedRevision) {
    if (this.error) throw Object.assign(new Error(this.error),{code:'STORAGE_FAILURE'})
    if (expectedRevision!==this.revision) throw Object.assign(new Error('待办已变化，请核对后重试；草稿已保留'),{code:'CONFLICT'})
    const next=structuredClone(this.state)
    const row=next.todos.find(t=>t.id===payload.id)
    const requireRow=()=>{if(!row)throw new Error('待办已不存在');return row}
    const now=this.now()
    switch(type) {
      case 'create': next.todos.unshift({id:this.newId(),...fields(payload),done:false,completedAt:null}); break
      case 'edit': {
        const target=requireRow(); if(target.deletedAt)throw new Error('待办已删除')
        Object.assign(target,fields(payload));break
      }
      case 'complete': {
        const target=requireRow();if(target.deletedAt || typeof payload.done!=='boolean')throw new Error('完成操作无效')
        target.done=payload.done;target.completedAt=payload.done?localDay(now):null;break
      }
      case 'trash': requireRow().deletedAt=new Date(now).toISOString();break
      case 'restore': delete requireRow().deletedAt;break
      case 'delete':
        if(!requireRow().deletedAt)throw new Error('请先移入回收站')
        next.todos=next.todos.filter(t=>t.id!==payload.id);break
      case 'archive':
        next.archive=history(next.todos,next.archive)
        next.todos=next.todos.filter(t=>!t.done || t.deletedAt);break
      default: throw new Error('未知待办操作')
    }
    try {
      if(type==='archive')this.repository.archive(this.state,next)
      else this.repository.save(next.todos)
    } catch(cause) {
      // A journal can be partially committed. Reload before admitting another command.
      try {this.repository.recover();this.state=this.repository.load();this.revision++}
      catch {this.error='存储状态需要恢复，已停止写入。请退出后检查文件与磁盘空间。'}
      throw Object.assign(new Error('保存未确认，请核对记录后重试'),{code:'STORAGE_FAILURE',cause})
    }
    this.state=next; this.revision++
    return this.snapshot()
  }
}
module.exports={TodosService}
