const { TrackingModel } = require('../../domain/tracking/model')
const { timestamp } = require('../../domain/tracking/validation')
class TrackingService extends TrackingModel {
  constructor(repository, {now, newId}) {
    super(structuredClone(repository.data), {now, newId})
    this.repository = repository
    this.error = repository.error; this.notice = repository.notice
    this.needsFlush = false
  }
  snapshot() { return {...super.snapshot(), error:this.error, notice:this.notice} }
  commit(change) {
    if (this.error) throw new Error(this.error)
    const previous = this.data
    try {
      super.commit(change)
      this.repository.save(this.data)
    } catch (error) {
      this.data = previous
      throw error
    }
    return this.snapshot()
  }
  interrupt(reason) {
    try { this.pause(reason) }
    catch (error) {
      const current = this.current()
      if (current) {
        this.closeSegment(current, this.now()); current.status='paused'; current.pauseReason=reason
        this.data.revision = (this.data.revision || 0) + 1
      }
      this.needsFlush = true; this.notice = '暂停状态尚未保存：'+error.message
    }
    return this.snapshot()
  }
  checkpoint() {
    if (this.needsFlush) {
      const snapshot = this.commit(() => {})
      this.needsFlush = false
      return snapshot
    }
    if (this.current()?.status !== 'running') return this.snapshot()
    const previous = this.data.checkpointAt ? timestamp(this.data.checkpointAt) : this.now()
    const elapsed = this.now() - previous
    if (elapsed > 90000 || elapsed < 0) return this.pause('clock-gap', previous)
    return this.commit(() => {})
  }}
module.exports = { TrackingService }
