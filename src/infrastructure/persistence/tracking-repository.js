const fs = require('node:fs')
const { randomUUID } = require('node:crypto')
const { atomicSave } = require('./atomic')
const { TrackingModel } = require('../../domain/tracking/model')
const { ensureSessions } = require('../../domain/tracking/sessions')
const { validateDocument, timestamp, iso, nextMatterColor } = require('../../domain/tracking/validation')
class TrackingRepository extends TrackingModel {
  constructor(file, { now = Date.now, write = atomicSave } = {}) {
    super({version:2, sessionVersion:1, checkpointAt:null, activities:[], trash:[]}, {now, newId:randomUUID})
    this.file = file; this.write = write; this.error = null; this.notice = null
    this.load()
  }
  save(data) {
    try { this.write(this.file, data) }
    catch (cause) { throw Object.assign(new Error('保存活动失败，操作未提交。'+cause.message), {code:'STORAGE_FAILURE', cause}) }
  }
  load() {
    if (!fs.existsSync(this.file)) return
    try {
      const raw = JSON.parse(fs.readFileSync(this.file, 'utf8'))
      // Never downgrade a future schema using an older backup.
      if (![1, 2].includes(raw.version)) throw new Error('无法识别活动数据版本，请保留文件并使用兼容版本')
      validateDocument(raw)
      this.data = raw
    } catch (error) {
      if (error.message.includes('版本')) { this.error = error.message; return }
      try {
        const backup = JSON.parse(fs.readFileSync(`${this.file}.bak`, 'utf8'))
        validateDocument(backup)
        fs.copyFileSync(this.file, `${this.file}.damaged-${Date.now()}-${randomUUID()}`)
        this.data = backup
        this.notice = '活动文件损坏，已恢复上一份备份。损坏原件已保留，请检查最近的记录。'
      } catch {
        this.error = '活动数据读取失败，已停止写入以保护原文件。请备份 activity-log.json 后检查或恢复其 .bak 文件。'
        return
      }
    }
    const migrating = this.data.version === 1 || this.data.sessionVersion !== 1 || this.data.activities.some(activity=>!Array.isArray(activity.sessions) || !activity.color)
    if (migrating) {
      try {
        this.migrationBackup = `${this.file}.v${this.data.version}-backup-${Date.now()}-${randomUUID()}.json`
        atomicSave(this.migrationBackup, this.data)
        this.data.version = 2
        this.data.sessionVersion = 1
        for (const activity of this.data.activities) {
          ensureSessions(activity)
          if (!activity.color) activity.color = nextMatterColor(this.data)
        }
      } catch {
        this.error = '旧版活动备份失败，已停止迁移，请检查目录权限和磁盘空间。'
        return
      }
    }
    const current = this.current()
    let recovered = false
    if (current?.status === 'running') {
      const segment = current.segments[current.segments.length - 1]
      const checkpoint = this.data.checkpointAt ? timestamp(this.data.checkpointAt) : timestamp(segment.start)
      const cutoff = Math.max(timestamp(segment.start), Math.min(this.now(), checkpoint))
      this.closeSegment(current, cutoff)
      current.status = 'paused'
      current.pauseReason = 'interrupted'
      current.updatedAt = iso(cutoff)
      recovered = true
      this.notice = [this.notice, '上次运行意外中断，已恢复到最近检查点并暂停。检查点通常间隔 30 秒，可在时间轴中修正。'].filter(Boolean).join(' ')
    }
    if (migrating || recovered) {
      try { this.write(this.file, this.data) } catch { this.error = '迁移或恢复状态保存失败，请检查磁盘空间和目录权限。原文件和备份已保留。' }
    }
  }

}
module.exports = { TrackingRepository }
