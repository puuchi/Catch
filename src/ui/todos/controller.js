const {visibleTodos,history,deadlineLabel,localDay}=require('../../domain/todos')
const icons=require('../shared/icons')
const {validateField}=require('../shared/form-validation')
const element=(tag,attrs={},text)=>{
  const node=document.createElement(tag)
  for(const [name,value] of Object.entries(attrs))node.setAttribute(name,String(value))
  if(text!==undefined)node.textContent=text
  return node
}
const iconButton=(name,label,cls)=>{
  const button=element('button',{type:'button',class:cls,title:label,'aria-label':label})
  button.append(icons.create(name));return button
}
module.exports=function mountTodos(api) {
  const byId=id=>document.getElementById(id)
  const form=byId('todoComposer'),list=byId('todoList'),input=byId('todoInput'),notes=byId('todoNotes')
  const details=byId('todoComposerDetails'),options=byId('todoComposerOptions'),create=byId('createTodoBtn')
  let snapshot={todos:[],archive:[],revision:0},busy=false,historyMode=false,editing=null,urgent=false
  function accept(next) {
    if(next.revision<snapshot.revision)return
    snapshot=next
    if(!busy&&!editing)render()
  }
  async function command(type,payload={},expectedRevision=snapshot.revision) {
    if(busy)throw new Error('上一操作尚未完成')
    busy=true;form.inert=true;list.inert=true;byId('clearDone').disabled=true
    try {
      const result=await api.todoCommand({type,payload,expectedRevision,requestId:crypto.randomUUID()})
      if(result.state)accept(result.state)
      if(!result.ok)throw new Error(result.error)
    } finally {busy=false;form.inert=false;list.inert=false;byId('clearDone').disabled=false}
  }
  const showError=error=>{byId('statusText').textContent=error.message}
  function grow(field) {field.style.height='auto';field.style.height=Math.min(field.scrollHeight+2,120)+'px'}
  function openComposer() {
    if(busy)return
    details.hidden=false;options.hidden=false;form.classList.add('expanded');input.setAttribute('aria-expanded','true');grow(notes)
  }
  function resetComposer() {
    form.reset();urgent=false;byId('optUrgent').classList.remove('active');byId('optDeadlineLabel').classList.remove('active')
    details.hidden=true;options.hidden=true;form.classList.remove('expanded');input.setAttribute('aria-expanded','false')
    byId('todoCreateError').hidden=true;input.blur()
  }
  const draft=()=>({text:input.value,notes:notes.value,urgent,deadline:byId('deadlineInput').value || null})
  input.addEventListener('focus',openComposer)
  input.addEventListener('input',openComposer)
  notes.addEventListener('input',()=>grow(notes))
  form.addEventListener('keydown',event=>{
    if(event.isComposing)return
    if(event.key==='Escape'&&!busy){event.preventDefault();resetComposer()}
    if(event.key==='Enter'&&event.target===input){event.preventDefault();form.requestSubmit()}
  })
  form.addEventListener('submit',async event=>{
    event.preventDefault();if(busy)return
    create.textContent='正在创建…';create.disabled=true;byId('todoCreateError').hidden=true
    try{await command('create',draft());resetComposer();render()}
    catch(error){byId('todoCreateError').textContent=error.message;byId('todoCreateError').hidden=false}
    finally{create.textContent='创建';create.disabled=false}
  })
  byId('cancelTodoBtn').onclick=()=>{if(!busy)resetComposer()}
  byId('optUrgent').onclick=()=>{urgent=!urgent;byId('optUrgent').classList.toggle('active',urgent)}
  byId('deadlineInput').onchange=()=>byId('optDeadlineLabel').classList.toggle('active',!!byId('deadlineInput').value)
  function showEditor(row,todo) {
    if(busy)return
    editing=todo.id;render()
    let editRevision=snapshot.revision
    row=list.querySelector(`[data-id="${CSS.escape(todo.id)}"]`)
    const body=row.querySelector('.todo-content');body.replaceChildren();row.classList.add('editing')
    row.querySelectorAll('button,input[type=checkbox]').forEach(node=>node.disabled=true)
    const title=element('textarea',{class:'edit-input',rows:1,maxlength:80,'aria-label':'编辑待办内容'})
    const note=element('textarea',{class:'edit-notes',rows:1,maxlength:2000,'aria-label':'备注',placeholder:'备注（可选）'})
    title.value=todo.text;note.value=todo.notes || ''
    let editUrgent=!!todo.urgent
    const flag=iconButton('flag','紧急','edit-urgent icon-btn-sm');flag.classList.toggle('active',editUrgent)
    flag.onclick=()=>{editUrgent=!editUrgent;flag.classList.toggle('active',editUrgent)}
    const date=element('input',{type:'date',class:'edit-deadline-input','aria-label':'待办截止日期'})
    date.value=todo.deadline || ''
    const dateLabel=element('label',{class:'edit-deadline icon-btn-sm',title:'日期'});dateLabel.append(icons.create('calendar'),date)
    dateLabel.classList.toggle('active',!!date.value);date.onchange=()=>dateLabel.classList.toggle('active',!!date.value)
    const cancel=element('button',{type:'button',class:'edit-cancel'},'取消')
    const save=element('button',{type:'button',class:'edit-confirm'},'确定')
    const error=element('div',{class:'edit-error',role:'alert',hidden:''})
    const controls=element('div',{class:'edit-toolbar'}),buttons=element('div',{class:'edit-actions'})
    buttons.append(cancel,save);controls.append(flag,dateLabel,buttons)
    const wrapper=element('div',{class:'edit-row'});wrapper.append(title,note,controls,error);body.append(wrapper)
    cancel.onclick=()=>{if(!busy){editing=null;render()}}
    save.onclick=async()=>{
      if(busy || !validateField(date))return
      try{await command('edit',{id:todo.id,text:title.value,notes:note.value,urgent:editUrgent,deadline:date.value || null},editRevision);editing=null;render()}
      catch(e){editRevision=snapshot.revision;error.textContent=e.message;error.hidden=false}
    }
    title.oninput=()=>grow(title);note.oninput=()=>grow(note)
    wrapper.onkeydown=event=>{
      if(event.isComposing)return
      if(event.key==='Escape'){event.preventDefault();cancel.click()}
      if(event.key==='Enter'&&(event.target===title || event.ctrlKey || event.metaKey)){event.preventDefault();save.click()}
    }
    grow(title);grow(note);title.focus();title.setSelectionRange(title.value.length,title.value.length)
  }
  function todoRow(todo,archived=false) {
    const today=localDay(Date.now()),row=element('li',{class:'todo-item','data-id':todo.id})
    row.classList.toggle('done',todo.done);row.classList.toggle('urgent',!!todo.urgent&&!todo.done)
    const check=element('input',{type:'checkbox','aria-label':'完成待办'});check.checked=todo.done;check.disabled=archived
    check.onchange=async()=>{try{await command('complete',{id:todo.id,done:check.checked});render()}catch(e){check.checked=todo.done;showError(e)}}
    const body=element('div',{class:'todo-content'}),line=element('div',{class:'todo-main'}),title=element('span',{class:'todo-text'},todo.text)
    line.append(title);body.append(line)
    if(todo.notes)body.append(element('p',{class:'todo-notes'},todo.notes))
    if(todo.deadline&&!todo.done){const badge=element('span',{class:'deadline-badge'},deadlineLabel(todo.deadline,today));badge.classList.toggle('today',todo.deadline===today);badge.classList.toggle('overdue',todo.deadline<today);line.append(badge)}
    row.append(check,body)
    if(archived) {row.append(element('span',{class:'history-date'},todo.completedAt || ''));return row}
    title.onclick=()=>showEditor(row,todo)
    if(!todo.done) {
      const start=iconButton('play','开始计时：'+todo.text,'todo-start')
      start.onclick=()=>window.dispatchEvent(new CustomEvent('start-todo-activity',{detail:{id:todo.id,name:todo.text}}))
      row.append(start)
    }
    const remove=iconButton('close','移入回收站','todo-delete')
    remove.onclick=async()=>{try{await command('trash',{id:todo.id});editing=null;render()}catch(e){showError(e)}}
    row.append(remove);return row
  }
  function render() {
    list.replaceChildren();byId('historyBtn').hidden=historyMode;byId('clearDone').hidden=historyMode;byId('btnBack').hidden=!historyMode
    byId('statusText').textContent=String(snapshot.todos.filter(t=>!t.done&&!t.deletedAt).length)
    const rows=historyMode?history(snapshot.todos,snapshot.archive):visibleTodos(snapshot.todos,localDay(Date.now()))
    if(!rows.length) {
      const empty=element('li',{class:'empty-hint todo-empty'})
      empty.append(element('strong',{},historyMode?'暂无完成记录':'暂无待办，留点空白'),element('p',{},historyMode?'完成的待办会保留在这里。':'把想做的事记下来，慢慢完成。'));list.append(empty)
      return
    }
    let section=''
    for(const todo of rows) {
      const group=historyMode?(todo.completedAt || '未知日期').slice(0,7):todo.done?'今日已完成':''
      if(group && group!==section)list.append(element('li',{class:'section-divider'},group))
      section=group;list.append(todoRow(todo,historyMode))
    }
  }
  byId('historyBtn').onclick=()=>{editing=null;historyMode=true;render()}
  byId('btnBack').onclick=()=>{editing=null;historyMode=false;render()}
  byId('clearDone').onclick=async()=>{if(busy)return;try{await command('archive');render()}catch(e){showError(e)}}
  let pinned=true
  byId('btnPin').onclick=()=>{pinned=!pinned;byId('btnPin').classList.toggle('active',pinned);byId('btnPin').setAttribute('aria-pressed',String(pinned));api.setPin(pinned)}
  byId('btnMin').onclick=()=>api.minimize();byId('btnClose').onclick=()=>api.close()
  const unsubscribe=api.onTodos(accept)
  const refresh=()=>api.getTodos().then(accept).catch(showError)
  window.addEventListener('todos-changed',refresh)
  window.addEventListener('unload',unsubscribe,{once:true})
  void refresh()
}
