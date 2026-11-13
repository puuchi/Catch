const { TrackingRepository } = require('../../src/infrastructure/persistence/tracking-repository')
const { TrackingService } = require('../../src/application/tracking/service')
const { randomUUID } = require('node:crypto')
class ActivityStore extends TrackingService {
  constructor(file, options = {}) {
    const repo = new TrackingRepository(file, options)
    super(repo, {now:options.now || Date.now,newId:randomUUID})
    this.migrationBackup=repo.migrationBackup
  }
  set write(fn) { this.repository.write=fn }
  get write() { return this.repository.write }
}
module.exports = { ActivityStore, ...require('../../src/domain/tracking/validation'), ...require('../../src/infrastructure/persistence/atomic') }
