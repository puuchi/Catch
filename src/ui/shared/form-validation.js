let errorSequence = 0
const errors = new WeakMap()
const activeFields = new Set()

function clearFieldError(field) {
  const error = errors.get(field)
  if (!error) return
  field.removeAttribute('aria-invalid')
  const describedBy = (field.getAttribute('aria-describedby') || '').split(/\s+/).filter(id => id && id !== error.id)
  if (describedBy.length) field.setAttribute('aria-describedby', describedBy.join(' '))
  else field.removeAttribute('aria-describedby')
  error.remove()
  errors.delete(field)
  activeFields.delete(field)
}

function showFieldError(field, message) {
  clearFieldError(field)
  const error = document.createElement('p')
  error.id = `field-feedback-${++errorSequence}`
  error.className = 'field-feedback'
  error.setAttribute('role', 'alert')
  error.textContent = message
  const anchor = field.closest('[data-validation-anchor], .input-area, .activity-input-row, .editor-field, .edit-toolbar, .todo-composer') || field
  anchor.insertAdjacentElement('afterend', error)
  field.setAttribute('aria-invalid', 'true')
  field.setAttribute('aria-describedby', [field.getAttribute('aria-describedby'), error.id].filter(Boolean).join(' '))
  errors.set(field, error)
  activeFields.add(field)
}

function fieldMessage(field) {
  if (!field.willValidate) return ''
  const value = field.value
  const label = field.getAttribute('aria-label') || field.labels?.[0]?.textContent.trim() || '内容'
  if (field.validity.badInput) return `请填写有效的${label}`
  if (field.required && !value.trim()) return field.dataset.emptyMessage || `请填写${label}`
  if (field.maxLength > 0 && value.length > field.maxLength) return `最多填写 ${field.maxLength} 个字符`
  if (!field.validity.valid) return `请检查${label}的格式或范围`
  return ''
}

function validateField(field) {
  const message = fieldMessage(field)
  if (message) showFieldError(field, message)
  else clearFieldError(field)
  return !message
}

function clearFormErrors(form) {
  for (const field of form.querySelectorAll('input,textarea,select')) clearFieldError(field)
}

function mountValidation(root = document) {
  for (const form of root.querySelectorAll('form')) form.noValidate = true
  root.addEventListener('submit', event => {
    const form = event.target
    let first = null
    for (const field of form.querySelectorAll('input,textarea,select')) {
      if (!field.getClientRects().length) continue
      if (!validateField(field)) first ||= field
    }
    if (first) {
      event.preventDefault()
      event.stopImmediatePropagation()
      first.focus({preventScroll:true})
      first.scrollIntoView({block:'nearest'})
    }
  }, true)
  for (const type of ['input', 'change']) root.addEventListener(type, event => clearFieldError(event.target))
  root.addEventListener('pointerdown', event => {
    for (const field of activeFields) {
      if (!field.contains(event.target) && !errors.get(field)?.contains(event.target)) clearFieldError(field)
    }
  }, true)
  root.addEventListener('focusout', event => clearFieldError(event.target))
  root.addEventListener('keydown', event => {
    if (event.key === 'Escape') for (const field of activeFields) clearFieldError(field)
  })
  root.addEventListener('reset', event => clearFormErrors(event.target), true)
}

module.exports = {mountValidation, validateField, showFieldError, clearFieldError, clearFormErrors}
