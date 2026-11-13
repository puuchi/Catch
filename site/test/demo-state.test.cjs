const test = require('node:test')
const assert = require('node:assert/strict')
const {DemoState, duration} = require('../src/demo-state')

function setup() {
  let now = Date.parse('2026-11-13T12:00:00Z')
  let id = 0
  const state = new DemoState({now: () => now, newId: () => `demo-${++id}`})
  return {state, advance: ms => { now += ms }, elapsed: activity => duration(activity, now)}
}

test('seed records are paused, valid, and reset without retaining input', () => {
  const {state} = setup()
  assert.equal(state.todos.length, 3)
  assert.equal(state.current, undefined)
  assert.equal(state.total(), (94 * 60 + 754) * 1000)
  assert.equal(state.selected.todoId, 'intro')
  const todo = state.addTodo('  一条新待办  ')
  assert.equal(todo.text, '一条新待办')
  assert.throws(() => state.addTodo('   '))
  assert.throws(() => state.addTodo('字'.repeat(81)))
  state.startTodo(todo.id)
  state.progress('私人输入')
  state.reset()
  assert.equal(state.todos.length, 3)
  assert.equal(state.page, 'todos')
  assert.equal(state.current, undefined)
  assert.ok(!JSON.stringify(state.tracking.data).includes('私人输入'))
})

test('live clock resumes after seeding and excludes paused time', () => {
  const {state, advance, elapsed} = setup()
  state.startTodo('read')
  advance(5000)
  assert.equal(elapsed(state.current), 5000)
  state.toggleTimer()
  advance(10000)
  assert.equal(elapsed(state.selected), 5000)
  state.toggleTimer()
  advance(3000)
  state.toggleTimer()
  assert.equal(elapsed(state.selected), 8000)
  assert.equal(state.selected.sessions.length, 2)
})

test('switching tasks pauses the previous task and keeps one active timer', () => {
  const {state, advance, elapsed} = setup()
  state.startTodo('intro')
  advance(4000)
  state.startTodo('read')
  const intro = state.tracking.data.activities.find(item => item.todoId === 'intro')
  assert.equal(intro.status, 'paused')
  assert.equal(elapsed(intro), 758000)
  assert.equal(state.current.todoId, 'read')
  assert.equal(state.tracking.data.activities.filter(item => item.status === 'running').length, 1)
})

test('next step preserves progress and duration; ending leaves todo unfinished', () => {
  const {state, advance, elapsed} = setup()
  state.startTodo('read')
  advance(2000)
  state.progress('第一步')
  state.next('第一步完成')
  advance(3000)
  state.progress('第二步')
  state.end()
  const activity = state.selected
  assert.equal(activity.status, 'completed')
  assert.equal(elapsed(activity), 5000)
  assert.deepEqual(activity.sessions.map(item => item.progress), ['第一步完成', '第二步'])
  assert.equal(state.todos.find(item => item.id === 'read').done, false)
  state.toggleTodo('read')
  assert.equal(state.todos.find(item => item.id === 'read').done, true)
  assert.equal(state.current, undefined)
})

test('two demo instances have independent in-memory records', () => {
  const a = setup().state
  const b = setup().state
  a.toggleTodo('intro')
  a.startMatter('独立事项')
  assert.equal(b.todos[0].done, false)
  assert.equal(b.current, undefined)
  assert.ok(!b.tracking.data.activities.some(item => item.name === '独立事项'))
})

test('retained foreground and background switching match desktop behavior',()=>{
  const {state,advance}=setup()
  const a=state.selected.id
  const b=state.tracking.paused().find(item=>item.id!==a).id
  assert.equal(state.tracking.paused().filter(item=>item.id!==a).length,2)
  state.resume(a);advance(2000);state.toggleTimer()
  assert.equal(state.selected.id,a)
  state.resume(b)
  assert.equal(state.selected.id,b)
  assert.equal(state.tracking.get(a).status,'paused')
  state.end()
  assert.equal(state.selected.id,b)
  assert.equal(state.selected.status,'completed')
  state.end(a)
  assert.equal(state.selected.id,b)
})

test('date filtering shows the selected day and empty dates',()=>{
  const {state}=setup()
  const initial=state.dayTotal()
  state.moveDay(-1)
  assert.equal(state.dayTotal(),35*60000)
  assert.equal(state.visits().length,1)
  state.moveDay(1)
  assert.equal(state.dayTotal(),initial)
  state.selectDay('2026-11-10')
  assert.equal(state.visits().length,0)
  assert.equal(state.dayTotal(),0)
  assert.throws(()=>state.selectDay('2026-02-30'))
})

test('restarting preserves matter identity, history, totals and todo status while excluding ended time',()=>{
  const {state,advance,elapsed}=setup()
  state.startTodo('read')
  advance(5000)
  state.progress('原来的进展')
  state.end()
  const before=structuredClone(state.selected)
  advance(60000)
  state.restart()
  const resumed=state.current
  assert.equal(resumed.id,before.id)
  assert.equal(resumed.color,before.color)
  assert.equal(resumed.todoId,before.todoId)
  assert.equal(resumed.endTime,null)
  assert.equal(resumed.sessions.length,before.sessions.length+1)
  assert.deepEqual(resumed.sessions.slice(0,-1),before.sessions)
  assert.equal(resumed.sessions.at(-1).progress,'')
  assert.equal(elapsed(resumed),5000)
  state.restart() // A second click while already running must not add another visit.
  assert.equal(state.current.sessions.length,before.sessions.length+1)
  advance(3000)
  state.end()
  assert.equal(elapsed(state.selected),8000)
  assert.equal(state.todos.find(item=>item.id==='read').done,false)
})
