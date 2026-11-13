const {TrackingModel}=require('../../src/domain/tracking/model')
const {fields,localDay,visibleTodos}=require('../../src/domain/todos')
const {duration,clock,humanDuration}=require('../../src/domain/tracking/time')
const {localDate,dayBounds,timelineEntries,durationOpacity}=require('../../src/domain/tracking/time')
const {foreground,statusText}=require('../../src/ui/tracking/foreground')

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
    for(const [minutesAgo,minutes,name,progress] of [[102,25,'阅读与思考','记下了三个值得实践的想法'],[65,20,'整理本周计划','选出这周最想完成的几件事']]) {
      this.tracking.addSession({name,progress,startTime:new Date(now-minutesAgo*60000).toISOString(),endTime:new Date(now-(minutesAgo-minutes)*60000).toISOString()})
    }
    const yesterday=new Date(now)
    yesterday.setDate(yesterday.getDate()-1)
    yesterday.setHours(14,0,0,0)
    this.tracking.addSession({name:'梳理项目思路',progress:'把想做的功能整理成了三步',startTime:yesterday.toISOString(),endTime:new Date(+yesterday+35*60000).toISOString()})
    this.todos=[
      {id:'intro',text:'写好项目介绍',notes:'让第一次看到的人，也能轻松理解',urgent:true,done:false,deadline:null},
      {id:'read',text:'读完收藏的那篇文章',notes:'留下一两个自己的想法',urgent:false,done:false,deadline:null},
      {id:'walk',text:'出去走走，换个思路',notes:'',urgent:false,done:false,deadline:null}
    ]
    seedTime=now-42*60000
    this.tracking.start('整理参考资料')
    seedTime+=8*60000
    this.tracking.pause()
    seedTime=now-30*60000
    this.tracking.start('准备下一篇笔记')
    seedTime+=6*60000
    this.tracking.pause()
    seedTime=now-16*60000
    this.tracking.start(this.todos[0].text,this.todos[0].id)
    seedTime+=754000
    this.tracking.pause()
    seedTime=null
    this.focusId=this.tracking.paused().find(item=>item.todoId==='intro').id
    this.timelineFocus=null
    this.day=localDate(new Date(now))
    this.page='todos'
  }

  get current() {return this.tracking.current()}
  get selected() {return foreground(this.tracking.data.activities,this.focusId)}
  get list() {return visibleTodos(this.todos,localDay(this.now()))}

  addTodo(text) {
    const todo={id:this.newId(),...fields({text,notes:'',urgent:false,deadline:null}),done:false,completedAt:null}
    this.todos.unshift(todo)
    return todo
  }

  toggleTodo(id) {
    const todo=this.todos.find(item=>item.id===id)
    if(!todo)throw new Error('这条待办已不存在')
    todo.done=!todo.done
    todo.completedAt=todo.done?localDay(this.now()):null
  }

  startTodo(id) {
    const todo=this.todos.find(item=>item.id===id && !item.done)
    if(!todo)throw new Error('请先选择一条未完成的待办')
    const existing=this.tracking.data.activities.find(item=>item.todoId===id && item.status!=='completed')
    if(existing)this.tracking.resume(existing.id)
    else this.tracking.start(todo.text,todo.id)
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
  end(id=this.selected?.id) {if(id)this.tracking.end(id)}
  restart() {
    const activity=this.selected
    if(activity?.status!=='completed')return
    this.tracking.updateMatter(activity.id,{name:activity.name,status:'running'})
    this.focusId=activity.id
    this.page='current'
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
  selectDay(day) {dayBounds(day);this.day=day;this.timelineFocus=null}
  moveDay(offset) {
    const date=new Date(`${this.day}T12:00:00`)
    date.setDate(date.getDate()+offset)
    this.selectDay(localDate(date))
  }
  visits() {return timelineEntries(this.tracking.data.activities,this.day,this.now())}
  dayTotal() {return this.tracking.data.activities.reduce((sum,item)=>sum+duration(item,this.now(),dayBounds(this.day)),0)}
}

module.exports={DemoState,duration,clock,humanDuration,localDate,dayBounds,durationOpacity,statusText}
