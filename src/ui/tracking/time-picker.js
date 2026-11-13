module.exports=function createTimePicker({byId,editor,inputTime,localInput,setEditorTime,useNow}) {
  const timePicker = byId('compactTimePicker')
  const pickerParts = ['pickerYear','pickerMonth','pickerDay','pickerHour','pickerMinute','pickerSecond'].map(byId)
  let pickerInput = null
  let pickerTrigger = null

  function closeTimePicker(restoreFocus = false) {
    timePicker.hidden = true
    pickerParts.forEach(input => { input.disabled = true })
    pickerTrigger?.setAttribute('aria-expanded','false')
    if (restoreFocus) pickerTrigger?.focus({preventScroll:true})
    pickerInput = null
    pickerTrigger = null
  }

  function showTimePicker(input, trigger) {
    if (pickerInput === input && !timePicker.hidden) { closeTimePicker(true); return }
    closeTimePicker()
    pickerInput = input
    pickerTrigger = trigger
    let date
    try { date = new Date(inputTime(input)) } catch { date = new Date() }
    const values = [date.getFullYear(),date.getMonth()+1,date.getDate(),date.getHours(),date.getMinutes(),date.getSeconds()]
    pickerParts.forEach((part,index) => { part.disabled = false; part.value = values[index] })
    byId('compactTimeTitle').textContent = input.id === 'editActivityStart' ? '开始时间' : '结束时间'
    byId('compactTimeError').textContent = ''
    trigger.setAttribute('aria-expanded','true')
    timePicker.hidden = false
    const anchor = input.getBoundingClientRect()
    const bounds = timePicker.getBoundingClientRect()
    timePicker.style.left = `${Math.max(8,Math.min(anchor.left,innerWidth-bounds.width-8))}px`
    const top = anchor.bottom+bounds.height+4 <= innerHeight-8 ? anchor.bottom+4 : anchor.top-bounds.height-4
    timePicker.style.top = `${Math.max(8,Math.min(top,innerHeight-bounds.height-8))}px`
    pickerParts[0].focus({preventScroll:true})
  }

  function applyTimePicker() {
    if (!pickerInput) return
    const values = pickerParts.map(input => input.value)
    if (pickerParts.some(input => !input.value || !input.checkValidity() || !Number.isInteger(Number(input.value)))) {
      byId('compactTimeError').textContent = '请填写有效的日期与时间'
      return
    }
    const [year,month,day,hour,minute,second] = values.map((value,index) => value.padStart(index === 0 ? 4 : 2,'0'))
    const value = `${year}-${month}-${day}T${hour}:${minute}:${second}`
    const date = new Date(value)
    if (!Number.isFinite(+date) || localInput(date) !== value) {
      byId('compactTimeError').textContent = '请填写有效的日期与时间'
      return
    }
    const input = pickerInput
    setEditorTime(input,date)
    // Preserve the original timestamp when the displayed second was left unchanged.
    input.dispatchEvent(new Event('input',{bubbles:true}))
    input.dispatchEvent(new Event('change',{bubbles:true}))
    closeTimePicker()
    input.focus({preventScroll:true})
  }

  byId('chooseSessionStart').addEventListener('click',() => showTimePicker(byId('editActivityStart'),byId('chooseSessionStart')))
  byId('chooseSessionEnd').addEventListener('click',() => showTimePicker(byId('editActivityEnd'),byId('chooseSessionEnd')))
  byId('sessionStartNow').addEventListener('click',() => useNow(byId('editActivityStart')))
  byId('sessionEndNow').addEventListener('click',() => useNow(byId('editActivityEnd')))
  byId('applyCompactTime').addEventListener('click',applyTimePicker)
  byId('cancelCompactTime').addEventListener('click',() => closeTimePicker(true))
  document.addEventListener('pointerdown',event => {
    if (!timePicker.hidden && !timePicker.contains(event.target) && !pickerTrigger?.contains(event.target)) closeTimePicker()
  })
  editor.addEventListener('keydown',event => {
    if (timePicker.hidden) return
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); closeTimePicker(true) }
    if (event.key === 'Enter' && timePicker.contains(event.target) && event.target.tagName === 'INPUT') {
      event.preventDefault()
      applyTimePicker()
    }
  })
  byId('editorBody').addEventListener('scroll',() => closeTimePicker())
  window.addEventListener('resize',() => closeTimePicker())


return {closeTimePicker}
}
