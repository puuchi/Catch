const {plants,stages}=require('./demo-state')
const {nextRepeatDate,repeatNames}=require('./repeat')
const {button,element,compact}=require('./dom')
const {stages:cropStages}=require('../../frontend/public/assets/plants/SOURCE.json')

const resourceTypes={file:'文件',folder:'文件夹',web:'网页'}
const samples={file:['方案草稿','C:\\Projects\\Catch\\方案.md'],folder:['项目资料','C:\\Projects\\Catch'],web:['参考网页','https://example.com/']}

function showPlant(sprite,plant,stage,size) {
  const sprout=plant==='sprout'
  const column=sprout?[0,8,17,26,35][stage]:cropStages[plant].columns[stage]
  const row=sprout?0:cropStages[plant].row
  Object.assign(sprite.style,{
    width:`${size}px`,height:`${size}px`,
    backgroundImage:`url('${sprout?'assets/sprout/growing-plant-sprite.png':'assets/plants/Crop_Spritesheet.png'}')`,
    backgroundSize:`${size*(sprout?61:12)}px ${size*(sprout?1:10)}px`,
    backgroundPosition:`-${size*column}px -${size*row}px`,
    imageRendering:sprout?'auto':'pixelated',
  })
}

module.exports=function mountControls(state,render,announce) {
  const byId=id=>document.getElementById(id)
  const resourcesDialog=byId('demo-resources-dialog')
  const repeatDialog=byId('demo-repeat-dialog')
  const growthDialog=byId('demo-growth-dialog')
  const finishDialog=byId('demo-finish-dialog')
  const resourceForm=byId('demo-resource-form')
  const resourceKind=byId('demo-resource-kind')
  const resourceName=byId('demo-resource-name')
  const resourceTarget=byId('demo-resource-target')
  const repeatKind=byId('demo-repeat-kind')
  const repeatStart=byId('demo-repeat-start')
  const repeatInterval=byId('demo-repeat-interval')
  let ownerId, editedResource, repeatTodo, finishAction, previewPlant='sprout'

  for(const close of document.querySelectorAll('[data-dialog-close]')) {
    close.addEventListener('click',()=>close.closest('dialog').close())
  }

  function feedback(id,message='') {
    const target=byId(id)
    target.textContent=message
    target.hidden=!message
  }

  function focusTodoControl(id,control) {
    document.querySelector(`[data-todo-id="${CSS.escape(id)}"] [data-demo-control="${control}"]`)?.focus()
  }

  function renderResourceLinks(owner,target) {
    target.replaceChildren()
    for(const resource of state.resourcesFor(owner)) {
      const link=element(resource.kind==='web'?'a':'span','demo-resource-link',resource.name)
      link.title=`${resourceTypes[resource.kind]} · ${resource.target}`
      if(resource.kind==='web') {
        link.href=resource.target
        link.target='_blank'
        link.rel='noopener noreferrer'
      }
      target.append(link)
    }
  }

  function updateResourceHint() {
    const [name,address]=samples[resourceKind.value]
    resourceName.placeholder=name
    resourceTarget.placeholder=address
    byId('demo-resource-hint').textContent=resourceKind.value==='web'?'关联后可点击名称打开网页。':'可填写示例路径。网页演示仅记录路径文字。'
    feedback('demo-resource-error')
  }

  function editResource(resource) {
    editedResource=resource?.id
    resourceKind.value=resource?.kind ?? 'file'
    resourceName.value=resource?.name ?? ''
    resourceTarget.value=resource?.target ?? ''
    byId('demo-resource-cancel').hidden=!resource
    byId('demo-resource-submit').textContent=resource?'保存修改':'添加资料'
    updateResourceHint()
  }

  function renderResourceEditor() {
    const list=byId('demo-resource-items')
    list.replaceChildren()
    for(const resource of state.resourcesFor(ownerId)) {
      const row=element('div','demo-resource-editor-row')
      const copy=element('div')
      copy.append(element('strong','',resource.name),element('small','',`${resourceTypes[resource.kind]} · ${resource.target}`))
      const edit=element('button','demo-secondary','修改')
      const remove=element('button','demo-secondary','移除')
      edit.type=remove.type='button'
      edit.setAttribute('aria-label',`修改资料：${resource.name}`)
      remove.setAttribute('aria-label',`移除资料：${resource.name}`)
      edit.addEventListener('click',()=>{editResource(resource);resourceName.focus()})
      remove.addEventListener('click',()=>{
        state.removeResource(ownerId,resource.id)
        if(editedResource===resource.id)editResource()
        renderResourceEditor();render();resourceKind.focus();announce('已移除资料')
      })
      row.append(copy,edit,remove)
      list.append(row)
    }
  }

  function openResources(id) {
    ownerId=id
    editResource()
    renderResourceEditor()
    resourcesDialog.showModal()
    resourceKind.focus()
  }

  resourcesDialog.addEventListener('close',()=>{
    if(state.page==='todos')focusTodoControl(ownerId,'resources')
    else byId('current-resource-button').focus()
  })

  resourceKind.addEventListener('change',updateResourceHint)
  byId('demo-resource-cancel').addEventListener('click',()=>{editResource();resourceKind.focus()})
  resourceForm.addEventListener('submit',event=>{
    event.preventDefault()
    try {
      state.saveResource(ownerId,{id:editedResource,kind:resourceKind.value,name:resourceName.value,target:resourceTarget.value})
      announce(editedResource?'已保存资料修改':'已关联资料')
      editResource();renderResourceEditor();render();resourceKind.focus()
    } catch(error) {feedback('demo-resource-error',error.message)}
  })

  function updateRepeat() {
    const kind=repeatKind.value, start=repeatStart.value
    const rule=state.repeatFor(repeatTodo)
    const interval=repeatInterval.valueAsNumber
    const valid=kind!=='every'||Number.isInteger(interval)&&interval>=1&&interval<=3650
    byId('demo-repeat-interval-field').hidden=kind!=='every'
    byId('demo-repeat-stop').hidden=!rule
    byId('demo-repeat-preview').hidden=!rule||rule.todoId!==repeatTodo.id||!repeatTodo.done
    byId('demo-repeat-note').textContent=rule&&rule.kind===kind&&rule.start===start&&(kind!=='every'||rule.interval===interval)
      ? `下次安排在 ${rule.next}，同一规则只保留一条未完成待办。`
      : start&&valid?`每次按计划生成；首个日期是 ${nextRepeatDate({start,kind,interval})}。`:''
    feedback('demo-repeat-error')
  }

  function openRepeat(todo) {
    repeatTodo=todo
    const rule=state.repeatFor(todo)
    repeatKind.value=rule?.kind ?? 'daily'
    repeatStart.value=rule?.start ?? state.today
    repeatInterval.value=rule?.interval ?? 2
    updateRepeat()
    repeatDialog.showModal()
    repeatKind.focus()
  }

  repeatDialog.addEventListener('close',()=>focusTodoControl(repeatTodo.id,'repeat'))

  for(const input of [repeatKind,repeatStart,repeatInterval])input.addEventListener('input',updateRepeat)
  byId('demo-repeat-form').addEventListener('submit',event=>{
    event.preventDefault()
    try {
      state.setRepeat(repeatTodo,repeatKind.value,repeatStart.value,repeatInterval.valueAsNumber)
      repeatDialog.close();render();announce('已保存重复规则')
    } catch(error) {feedback('demo-repeat-error',error.message)}
  })
  byId('demo-repeat-stop').addEventListener('click',()=>{
    state.stopRepeat(repeatTodo);repeatDialog.close();render();announce('已停止重复')
  })
  byId('demo-repeat-preview').addEventListener('click',()=>{
    const rule=state.repeatFor(repeatTodo)
    if(rule && state.emitRepeat(rule)) {
      repeatDialog.close();render();announce('已在演示中生成下一次待办')
    }
  })

  const choices=[]
  for(const [plant,name] of Object.entries(plants)) {
    const choice=element('button','demo-plant-choice')
    const sprite=element('span','demo-plant-sprite')
    choice.type='button'
    showPlant(sprite,plant,4,22)
    choice.append(sprite,element('span','',name))
    choice.addEventListener('click',()=>{previewPlant=plant;renderGrowth()})
    byId('demo-plant-options').append(choice)
    choices.push([plant,choice])
  }
  const stageItems=stages.map(label=>{
    const item=element('li')
    item.append(element('span','demo-plant-sprite'),element('span','',label))
    byId('demo-growth-stages').append(item)
    return item
  })

  function renderGrowth() {
    const growth=state.growth
    const floating=byId('demo-floating-plant')
    byId('growth-bar-time').textContent=compact(growth.invested)
    showPlant(floating,growth.plant,growth.chosen?growth.stage:0,44)
    if(!growthDialog.open)return
    const viewing=previewPlant===growth.plant && growth.chosen
    const stage=viewing?growth.stage:0
    showPlant(byId('demo-plant-preview'),previewPlant,stage,52)
    byId('demo-plant-name').textContent=plants[previewPlant]
    byId('demo-growth-status').textContent=viewing
      ? `今日投入 ${compact(growth.invested)} · ${stages[stage]}${growth.nextMinutes?`，距下阶段 ${growth.nextMinutes} 分钟`:''}`
      : growth.chosen?'浏览植物阶段':'选定后，植物随今天的投入生长'
    byId('demo-select-plant').hidden=growth.chosen
    byId('demo-grow').hidden=!viewing || stage===4
    for(const [plant,choice] of choices)choice.setAttribute('aria-pressed',String(plant===previewPlant))
    stageItems.forEach((item,index)=>{
      showPlant(item.firstElementChild,previewPlant,index,26)
      if(viewing&&index===stage)item.setAttribute('aria-current','step')
      else item.removeAttribute('aria-current')
    })
  }

  function openGrowth() {
    previewPlant=state.growth.plant
    byId('demo-goal-minutes').value=state.goalMinutes
    growthDialog.showModal();renderGrowth()
  }

  byId('growth-open').addEventListener('click',openGrowth)
  byId('demo-select-plant').addEventListener('click',()=>{state.choosePlant(previewPlant);renderGrowth();announce('已选定今天的植物')})
  byId('demo-grow').addEventListener('click',()=>{state.simulateGrowth();renderGrowth();announce('已模拟投入 30 分钟')})
  byId('demo-goal-form').addEventListener('submit',event=>{
    event.preventDefault()
    state.setGoal(byId('demo-goal-minutes').valueAsNumber);renderGrowth();announce('已保存每日目标')
  })

  function confirmFinish(action) {finishAction=action;finishDialog.showModal()}

  for(const [id,complete] of [['demo-finish-only',false],['demo-finish-complete',true]]) {
    byId(id).addEventListener('click',()=>{finishDialog.close();finishAction(complete)})
  }

  function todoActions(todo) {
    const actions=element('div','demo-todo-actions')
    const resources=button('paperclip',`关联资料：${todo.text}`,'demo-chip')
    resources.dataset.demoControl='resources'
    resources.append(element('span','','资料'))
    resources.addEventListener('click',()=>openResources(todo.id))
    actions.append(resources)
    const rule=state.repeatFor(todo)
    if(!todo.done || rule?.todoId===todo.id) {
      const repeat=button('repeat',`设置重复：${todo.text}`,'demo-chip')
      repeat.dataset.demoControl='repeat'
      repeat.append(element('span','',rule?repeatNames[rule.kind]:'重复'))
      repeat.addEventListener('click',()=>openRepeat(todo))
      actions.append(repeat)
    }
    return actions
  }

  function reset() {
    for(const dialog of [resourcesDialog,repeatDialog,growthDialog,finishDialog])if(dialog.open)dialog.close()
  }

  return {todoActions,renderResourceLinks,openResources,openGrowth,renderGrowth,confirmFinish,reset}
}
