const {TrackingModel}=require('./demo-model/tracking/model')
const {fields,localDay,visibleTodos}=require('./demo-model/todos')
const {duration,clock,humanDuration}=require('./demo-model/tracking/time')
const {localDate,dayBounds,timelineEntries,durationOpacity}=require('./demo-model/tracking/time')
const {foreground,statusText}=require('./demo-model/foreground')
const {nextRepeatDate,repeatNames}=require('./repeat')

const plants={sprout:'绿芽',tulip:'郁金香',sunflower:'向日葵',corn:'玉米',strawberry:'草莓'}
const stages=['初始','萌芽','生长','茂盛','成熟']
const growthThresholds=[0.1,0.3,0.6,1]

class DemoState {
  constructor({now=Date.now,newId=()=>globalThis.crypto.randomUUID()}={}) {
    this.now=now
    this.newId=newId
    this.reset()
  }

  reset() {
    const now=this.now()
    let seedTime=null
    this.tracking=new TrackingModel({version:2,sessionVersion:1,checkpointAt:null,activities:[],trash:[]},{now:()=>seedTime ?? this.now(),newId:this.newId})
    for(const [minutesAgo,minutes,name,progress] of [[102,25,'检查归还区的书','挑出三本需要修补书角的书'],[65,20,'画出借阅流程','确定登记、取书和归还的动线']]) {
      this.tracking.addSession({name,progress,startTime:new Date(now-minutesAgo*60000).toISOString(),endTime:new Date(now-(minutesAgo-minutes)*60000).toISOString()})
    }
    const yesterday=new Date(now)
    yesterday.setDate(yesterday.getDate()-1)
    yesterday.setHours(14,0,0,0)
    this.tracking.addSession({name:'整理书架的分类',progress:'把儿童读物移到最下面两层',startTime:yesterday.toISOString(),endTime:new Date(+yesterday+35*60000).toISOString()})
    this.todos=[
      {id:'intro',text:'写社区书架的借阅指引',notes:'',urgent:false,done:false,deadline:null},
      {id:'read',text:'给新到的绘本贴标签',notes:'',urgent:false,done:false,deadline:null},
      {id:'walk',text:'清点待归还的漂流书',notes:'',urgent:false,done:false,deadline:null}
    ]
    seedTime=now-42*60000
    this.tracking.start('核对周六读书会流程')
    seedTime+=8*60000
    this.tracking.pause()
    seedTime=now-30*60000
    this.tracking.start('制作借阅登记表')
    seedTime+=6*60000
    this.tracking.pause()
    seedTime=now-16*60000
    this.tracking.start(this.todos[0].text,this.todos[0].id)
    seedTime+=754000
    this.tracking.pause()
    seedTime=null
    this.focusId=this.tracking.paused().find(item=>item.todoId==='intro').id
    const draft=this.tracking.get(this.focusId)
    this.tracking.setProgress(draft.id,draft.sessions.at(-1).id,'写好了借还规则，下一步补充开放时间。')
    this.hiddenCompletedId=null
    this.timelineFocus=null
    this.processId=null
    this.day=localDate(new Date(now))
    this.page='todos'
    this.resources=new Map()
    this.repeats=new Map()
    this.growthDays=new Map()
    this.goalMinutes=120
  }

  get current() {return this.tracking.current()}
  get selected() {
    const selected=foreground(this.tracking.data.activities,this.focusId)
    return selected?.id===this.hiddenCompletedId ? null : selected
  }
  get today() {return localDay(this.now())}
  get list() {
    for(const rule of this.repeats.values()) {
      const todo=this.todos.find(item=>item.id===rule.todoId)
      if(todo.done && rule.next<=this.today)this.emitRepeat(rule,rule.next,this.today)
    }
    return visibleTodos(this.todos,this.today)
  }

  addTodo(text) {
    const todo={id:this.newId(),...fields({text,notes:'',urgent:false,deadline:null}),done:false,completedAt:null}
    this.todos.unshift(todo)
    return todo
  }

  toggleTodo(id) {
    const todo=this.todos.find(item=>item.id===id)
    if(!todo)throw new Error('这条待办已不存在')
    const rule=this.repeats.get(todo.repeatId)
    if(todo.done && rule && rule.todoId!==id)throw new Error('这个重复待办已有新的一次')
    todo.done=!todo.done
    todo.completedAt=todo.done?localDay(this.now()):null
    if(todo.done && rule?.todoId===id)rule.next=nextRepeatDate(rule,[this.today,todo.deadline].sort().at(-1))
  }

  repeatFor(todo) {return this.repeats.get(todo.repeatId)}

  setRepeat(todo,kind,start,interval,weekdays,monthDay) {
    dayBounds(start)
    if(!Object.hasOwn(repeatNames,kind))throw new Error('请选择重复方式')
    if(kind==='every' && (!Number.isInteger(interval) || interval<1 || interval>3650))throw new Error('间隔天数须为 1–3650')
    const rule={id:todo.repeatId ?? this.newId(),todoId:todo.id,kind,start,interval,weekdays,monthDay}
    const date=nextRepeatDate(rule)
    rule.next=nextRepeatDate(rule,[this.today,date].sort().at(-1))
    this.repeats.set(rule.id,rule)
    todo.repeatId=rule.id
    todo.deadline=date
  }

  stopRepeat(todo) {
    this.repeats.delete(todo.repeatId)
  }

  emitRepeat(rule,date=rule.next,through=date) {
    const previous=this.todos.find(item=>item.id===rule.todoId)
    if(!previous.done)return
    const todo=this.addTodo(previous.text)
    Object.assign(todo,{notes:previous.notes,urgent:previous.urgent,repeatId:rule.id,deadline:date})
    this.resources.set(todo.id,this.resourcesFor(previous.id).map(resource=>({...resource})))
    rule.todoId=todo.id
    rule.next=nextRepeatDate(rule,through)
    return todo
  }

  resourcesFor(id) {return this.resources.get(id) ?? []}

  saveResource(owner,{id,kind,name,target}) {
    name=name.trim();target=target.trim()
    if(!name || !target)throw new Error('请填写资料名称和地址')
    if(kind==='web' && (!URL.canParse(target) || !['https:','http:'].includes(new URL(target).protocol))) {
      throw new Error('请输入以 http:// 或 https:// 开头的网页地址')
    }
    const resources=this.resourcesFor(owner)
    const resource={id:id ?? this.newId(),kind,name,target}
    this.resources.set(owner,id ? resources.map(item=>item.id===id ? resource : item) : [...resources,resource])
  }

  removeResource(owner,id) {
    this.resources.set(owner,this.resourcesFor(owner).filter(item=>item.id!==id))
  }

  get growth() {
    const day=this.growthDays.get(this.today)
    const invested=this.totalOn(this.today)+(day?.simulated ?? 0)
    const goal=this.goalMinutes ?? 60
    const stage=growthThresholds.filter(threshold=>invested>=goal*60000*threshold).length
    const nextMinutes=stage===4?0:Math.ceil(goal*growthThresholds[stage]-invested/60000)
    return {plant:day?.plant ?? 'sprout',chosen:!!day?.plant,invested,stage,nextMinutes}
  }

  choosePlant(plant) {
    if(!Object.hasOwn(plants,plant) || this.growth.chosen)return
    this.growthDays.set(this.today,{plant,simulated:0})
  }

  simulateGrowth() {
    const day=this.growthDays.get(this.today)
    if(day?.plant)day.simulated+=30*60000
  }

  setGoal(minutes) {
    if(minutes!==null&&(!Number.isInteger(minutes)||minutes<1||minutes>1440))throw new Error('每日目标须为 1–1440 分钟')
    this.goalMinutes=minutes
  }

  startTodo(id) {
    const todo=this.todos.find(item=>item.id===id && !item.done)
    if(!todo)throw new Error('请先选择一条未完成的待办')
    const existing=this.tracking.data.activities.find(item=>item.todoId===id && item.status!=='completed')
    if(existing)this.tracking.resume(existing.id)
    else {
      this.tracking.start(todo.text,todo.id)
      this.resources.set(this.current.id,this.resourcesFor(todo.id).map(resource=>({...resource})))
    }
    this.focusId=this.current.id
    this.page='current'
  }

  startMatter(name) {
    this.tracking.start(name)
    this.focusId=this.current.id
    this.page='current'
  }

  toggleTimer() {
    const selected=this.selected
    if(!selected || selected.status==='completed')return
    if(selected.status==='running')this.tracking.pause()
    else this.tracking.resume(selected.id)
  }

  resume(id) {this.tracking.resume(id);this.focusId=id;this.page='current'}
  end(id=this.selected?.id,{completeTodo=false,progress}={}) {
    const activity=this.tracking.get(id)
    if(!activity || activity.status==='completed')return
    if(progress!==undefined)this.tracking.setProgress(id,activity.sessions.at(-1).id,progress)
    this.tracking.end(id)
    const todo=this.todos.find(item=>item.id===activity.todoId && !item.done)
    if(completeTodo && todo)this.toggleTodo(todo.id)
  }
  closeCompleted() {
    if(this.selected?.status==='completed')this.hiddenCompletedId=this.selected.id
  }
  showProcess() {
    this.processId=this.selected?.id
    this.timelineFocus=null
    this.page='timeline'
  }
  progress(text) {
    const selected=this.selected
    const session=selected?.sessions.at(-1)
    if(!session)throw new Error('请先开始一件事情')
    this.tracking.setProgress(selected.id,session.id,text)
  }
  next(text) {
    if(!this.current)throw new Error('继续计时后，才能进入下一步')
    const selected=this.current
    this.tracking.nextSession(selected.id,selected.sessions.at(-1).id,text)
  }
  total() {return this.tracking.data.activities.reduce((sum,item)=>sum+duration(item,this.now()),0)}
  selectDay(day) {dayBounds(day);this.day=day;this.timelineFocus=null;this.processId=null}
  moveDay(offset) {
    const date=new Date(`${this.day}T12:00:00`)
    date.setDate(date.getDate()+offset)
    this.selectDay(localDate(date))
  }
  visits() {return timelineEntries(this.tracking.data.activities,this.day,this.now())}
  totalOn(day) {
    const bounds=dayBounds(day)
    return this.tracking.data.activities.reduce((sum,item)=>sum+duration(item,this.now(),bounds),0)
  }
  dayTotal() {return this.totalOn(this.day)}
}

module.exports={DemoState,duration,clock,humanDuration,localDate,dayBounds,durationOpacity,statusText,plants,stages}
