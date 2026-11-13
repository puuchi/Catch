const test=require('node:test')
const assert=require('node:assert/strict')
const {foreground,statusText}=require('../src/ui/tracking/foreground')

test('foreground retains a paused or finished matter until another one starts',()=>{
  const a={id:'a',status:'running',updatedAt:'2026-11-13T10:00:00Z'}
  const b={id:'b',status:'paused',updatedAt:'2026-11-13T11:00:00Z'}
  assert.equal(foreground([a,b],null).id,'a')
  a.status='paused'
  assert.equal(foreground([a,b],'a').id,'a')
  assert.equal(statusText(a),'准备继续')
  b.status='running'
  assert.equal(foreground([a,b],'a').id,'b')
  b.status='completed'
  assert.equal(foreground([a,b],'b').id,'b')
  assert.equal(statusText(b),'这件事告一段落')
  assert.equal(foreground([a],'b').id,'a')
})

test('initial display chooses the latest paused matter, never an arbitrary completed matter',()=>{
  const activities=[{id:'done',status:'completed',updatedAt:'2026-11-13T12:00:00Z'},
    {id:'old',status:'paused',updatedAt:'2026-11-13T10:00:00Z'},
    {id:'recent',status:'paused',updatedAt:'2026-11-13T11:00:00Z'}]
  assert.equal(foreground(activities,null).id,'recent')
  assert.equal(foreground([activities[0]],null),null)
  assert.equal(foreground([],null),null)
})
