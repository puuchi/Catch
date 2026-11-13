// @ts-check
/** @typedef {{id:string,text:string,notes?:string,done:boolean,urgent?:boolean,deadline?:string|null,completedAt?:string|null,deletedAt?:string}} Todo */
/** @param {unknown} value */
function calendarDate(value) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return false
  const date = new Date(value+'T12:00:00Z')
  return Number.isFinite(+date) && date.toISOString().slice(0,10) === value
}
/** @param {unknown} value @returns {Todo[]} */
function validateRecords(value) {
  if (!Array.isArray(value)) throw new Error('待办文件格式无效，已停止写入')
  const ids = new Set()
  for (const row of value) {
    if (!row || typeof row.id !== 'string' || !row.id || ids.has(row.id) || typeof row.text !== 'string' || typeof row.done !== 'boolean') throw new Error('待办记录无效或 ID 重复')
    if (row.notes !== undefined && typeof row.notes !== 'string') throw new Error('待办备注格式无效')
    if (row.deadline != null && !calendarDate(row.deadline)) throw new Error('截止日期无效')
    ids.add(row.id)
  }
  return value
}
/** @param {any} input */
function fields(input) {
  if (!input || typeof input.text !== 'string' || !input.text.trim() || input.text.trim().length > 80) throw new Error('待办内容须为 1–80 个字符')
  if (input.notes != null && (typeof input.notes !== 'string' || input.notes.length > 2000)) throw new Error('备注最多 2000 个字符')
  if (typeof input.urgent !== 'boolean') throw new Error('紧急标记无效')
  if (input.deadline != null && !calendarDate(input.deadline)) throw new Error('截止日期无效')
  return {text:input.text.trim(), notes:(input.notes || '').trim(), urgent:input.urgent, deadline:input.deadline || null}
}
/** @param {number} time */
function localDay(time) {
  const parts = new Intl.DateTimeFormat('en', {year:'numeric',month:'2-digit',day:'2-digit'}).formatToParts(time)
  const part = type => parts.find(p => p.type === type).value
  return `${part('year')}-${part('month')}-${part('day')}`
}
/** @param {Todo[]} rows @param {string} today */
function visibleTodos(rows, today) {
  const rank = item => item.done ? 3 : item.urgent ? 0 : item.deadline === today ? 1 : 2
  return rows.filter(item => !item.deletedAt && (!item.done || item.completedAt === today)).slice().sort((a,b) => rank(a)-rank(b))
}
/** @param {Todo[]} rows @param {Todo[]} archive */
function history(rows, archive) {
  const byId = new Map(archive.filter(t => !t.deletedAt).map(t => [t.id,t]))
  for (const row of rows) if (row.done && !row.deletedAt) byId.set(row.id,row)
  return Array.from(byId.values()).sort((a,b) => (b.completedAt || '').localeCompare(a.completedAt || ''))
}
/** @param {string} date @param {string} today */
function deadlineLabel(date, today) {
  const target = new Date(date+'T12:00:00Z')
  const days = Math.round((+target - +new Date(today+'T12:00:00Z')) / 86400000)
  const label = `${target.getUTCMonth()+1}月${target.getUTCDate()}日`
  return label + (days===0 ? ' · 今天截止' : days<0 ? ` · 已逾期${-days}天` : ` · ${days}天后`)
}
module.exports = {validateRecords, fields, localDay, visibleTodos, history, deadlineLabel}
