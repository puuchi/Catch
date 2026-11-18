const {DemoState}=require('./demo-state')

function createPreview(options) {
  const demo=new DemoState(options)
  const archive=[]
  let todosRevision=0, recurrencesRevision=0, growthRevision=0
  demo.saveResource('read',{kind:'file',name:'绘本标签模板',target:'C:\\社区书架\\绘本标签模板.pdf'})
  demo.setRepeat(demo.todos.find(todo=>todo.id==='walk'),'weekly',demo.today,undefined,[5])

  const resources=id=>demo.resourcesFor(id).map(({name,...resource})=>({...resource,title:name}))
  const withResources=row=>({...row,resources:resources(row.id)})
  const findTodo=id=>demo.todos.find(todo=>todo.id===id) || archive.find(todo=>todo.id===id)
  const ruleTodo=rule=>findTodo(rule.todoId)
  const repeatFields=({text,notes,urgent,deadline})=>({text,notes,urgent,deadline})
  const schedules={
    daily:()=>({kind:'daily'}),
    weekly:rule=>({kind:'weekly',weekdays:rule.weekdays}),
    every:rule=>({kind:'every',days:rule.interval}),
    monthly:rule=>({kind:'monthly',day:rule.monthDay}),
  }
  const unsupported=()=>{throw Error('这项操作可在桌面版使用')}

  function todoSnapshot(todo) {
    const result=withResources(todo)
    delete result.repeatId
    if(todo.repeatId)result.recurrence={templateId:todo.repeatId,scheduledOn:todo.deadline}
    return result
  }

  function snapshot() {
    const tracking=demo.tracking.snapshot()
    tracking.activities=tracking.activities.map(withResources)
    return {
      tracking,
      todos:{revision:todosRevision,todos:demo.todos.map(todoSnapshot),archive:archive.map(todoSnapshot)},
      recurrences:{version:1,revision:recurrencesRevision,templates:Array.from(demo.repeats.values(),rule=>({
        id:rule.id,fields:repeatFields(ruleTodo(rule)),resources:resources(rule.todoId),schedule:schedules[rule.kind](rule),
        startsOn:rule.start,nextOn:rule.next,lastEmitted:ruleTodo(rule).deadline,stopped:false,
      }))},
      growth:{version:1,revision:growthRevision,goalMinutes:demo.goalMinutes,preferredPlant:'tulip',days:{}},
      trackingNotice:null,
      storageBlocked:false,
    }
  }

  function growth() {
    const view=demo.growth
    return {
      day:demo.today,plant:view.chosen?view.plant:'tulip',selected:view.chosen,investedMs:view.invested,
      goalMinutes:demo.goalMinutes,stage:view.stage,reached:demo.goalMinutes!==null&&view.stage===4,celebrated:false,
    }
  }

  function newTodo(input,attachments=[]) {
    const todo=demo.addTodo(input.text)
    Object.assign(todo,{notes:input.notes || '',deadline:input.deadline ?? null,urgent:!!input.urgent})
    for(const resource of attachments)demo.saveResource(todo.id,{...resource,name:resource.title,id:undefined})
    return todo
  }

  function repeat(todo,schedule,date) {
    demo.setRepeat(todo,schedule.kind,date,schedule.days,schedule.weekdays,schedule.day)
  }

  const commands={
    todos:{
      create:payload=>newTodo(payload),
      'create-with-resources':payload=>newTodo(payload.fields,payload.resources),
      edit:payload=>Object.assign(findTodo(payload.id),repeatFields(payload)),
      complete:({id,done})=>{if(findTodo(id).done!==done)demo.toggleTodo(id)},
      trash:({id})=>{findTodo(id).deletedAt=new Date(demo.now()).toISOString()},
      restore:({id})=>{delete findTodo(id).deletedAt},
      archive:()=>{
        archive.push(...demo.todos.filter(todo=>todo.done&&!todo.deletedAt&&!todo.repeatId))
        demo.todos=demo.todos.filter(todo=>!todo.done||todo.deletedAt||todo.repeatId)
      },
      'restore-archive':({id})=>{
        const todo=findTodo(id)
        const index=archive.indexOf(todo)
        if(index>=0)demo.todos.unshift(...archive.splice(index,1))
        todo.done=false;todo.completedAt=null
      },
    },
    tracking:{
      start:({name,todoId})=>todoId?demo.startTodo(todoId):demo.startMatter(name),
      resume:({id})=>demo.resume(id),
      pause:()=>demo.tracking.pause(),
      'set-progress':({activityId,sessionId,progress})=>demo.tracking.setProgress(activityId,sessionId,progress),
      'next-session':({activityId,sessionId,progress})=>demo.tracking.nextSession(activityId,sessionId,progress),
      'add-session':payload=>demo.tracking.addSession(payload),
      'edit-session':payload=>demo.tracking.editSession(payload),
      'update-matter':({id,...payload})=>demo.tracking.updateMatter(id,payload),
      remove:({id})=>demo.tracking.remove(id),
      'restore-trash':({id})=>demo.tracking.restoreTrash(id),
      'delete-trash':({id})=>demo.tracking.deleteTrash(id),
    },
    growth:{choose:({plant})=>demo.choosePlant(plant),'set-goal':({minutes})=>demo.setGoal(minutes)},
    recurrence:{
      create:({fields,resources:attachments,schedule,startsOn})=>repeat(newTodo(fields,attachments),schedule,startsOn),
      convert:({id,schedule,startsOn})=>repeat(findTodo(id),schedule,startsOn),
      update:({id,schedule,fields})=>{
        const rule=demo.repeats.get(id), todo=ruleTodo(rule)
        Object.assign(todo,fields)
        repeat(todo,schedule,rule.start)
      },
      stop:({id})=>demo.stopRepeat(ruleTodo(demo.repeats.get(id))),
    },
    resources:{
      attach:({owner,kind,target,title})=>{
        if(demo.resourcesFor(owner.id).some(item=>item.target===target))throw Error('这份资料已关联')
        const name=title || (kind==='web'?new URL(target).host:target.split(/[\\/]/).at(-1))
        demo.saveResource(owner.id,{kind,name,target})
      },
      relocate:({owner,resourceId,target,title})=>{
        const resource=demo.resourcesFor(owner.id).find(item=>item.id===resourceId)
        demo.saveResource(owner.id,{...resource,name:title || resource.name,target})
      },
      remove:({owner,resourceId})=>demo.removeResource(owner.id,resourceId),
    },
  }

  function command(group,command) {
    let error
    try {
      if(group==='finish') {
        demo.end(command.activityId,command)
        todosRevision++
      } else {
        const {type,payload}=command
        const operation=commands[group]?.[type] || unsupported
        operation(payload)
        if(group==='todos'||group==='recurrence'||group==='resources'&&payload.owner.kind==='todo')todosRevision++
        if(group==='resources'&&payload.owner.kind==='matter')demo.tracking.data.revision++
        if(group==='growth')growthRevision++
        if(group==='recurrence')recurrencesRevision++
      }
    } catch(cause) {error={code:'demo',message:cause.message}}
    return {ok:!error,error,state:snapshot(),now:demo.now(),growth:growth()}
  }

  function repeatDue() {
    return [...demo.repeats.values()].find(rule=>ruleTodo(rule)?.done)
  }

  function simulateRepeat() {
    const rule=repeatDue()
    if(rule&&demo.emitRepeat(rule)) {todosRevision++;recurrencesRevision++}
    return snapshot()
  }

  return {demo,snapshot,growth,command,repeatDue,simulateRepeat,resources}
}

module.exports={createPreview}
