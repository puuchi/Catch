const fs = require('node:fs')
const path = require('node:path')
const { atomicSave } = require('./atomic')
const { validateRecords } = require('../../domain/todos')

class TodosRepository {
  constructor(directory, {write = atomicSave, fault = (_stage) => {}} = {}) {
    this.directory=directory; this.write=write; this.fault=fault
    this.pending=path.join(directory,'catch-archive-pending.json')
    this.recover()
  }
  path(name) { return path.join(this.directory, name+'.json') }
  read(name) {
    const file=this.path(name)
    return fs.existsSync(file) ? validateRecords(JSON.parse(fs.readFileSync(file,'utf8'))) : []
  }
  load() { return {todos:this.read('todos'),archive:this.read('archive')} }
  save(todos) { this.write(this.path('todos'),todos) }
  recover() {
    if (!fs.existsSync(this.pending)) return
    const journal=JSON.parse(fs.readFileSync(this.pending,'utf8'))
    if (journal.version!==1 || !Array.isArray(journal.before) || !Array.isArray(journal.after)) throw new Error('归档恢复版本无效，请保留文件')
    validateRecords(journal.before); validateRecords(journal.after); validateRecords(journal.archiveBefore); validateRecords(journal.archiveAfter)
    const current=this.load()
    const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b)
    if ((!same(current.todos,journal.before)&&!same(current.todos,journal.after)) || (!same(current.archive,journal.archiveBefore)&&!same(current.archive,journal.archiveAfter))) throw new Error('归档期间数据被外部修改，已停止恢复')
    this.write(this.path('archive'),journal.archiveAfter); this.fault('archive')
    this.write(this.path('todos'),journal.after); this.fault('todos')
    fs.unlinkSync(this.pending); this.fault('cleanup')
  }
  archive(state, next) {
    this.write(this.pending,{version:1,before:state.todos,archiveBefore:state.archive,after:next.todos,archiveAfter:next.archive})
    this.fault('journal')
    this.recover()
  }
}
module.exports={TodosRepository}
