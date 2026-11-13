const path = require('path')
const fs = require('fs')

const APP_NAME = 'Catch'
const LEGACY_DATA_DIRECTORY = 'float-todo'

function configureAppIdentity(app) {
  const existingName = app.getName()
  const existingData = app.getPath('userData')
  const appData = app.getPath('appData')
  const normalize = value => {
    const resolved = path.resolve(value)
    return process.platform === 'win32' ? resolved.toLowerCase() : resolved
  }
  // Keep explicit profile overrides (tests/previews). Default installs continue
  // using the established data directory even if packaging supplies the new name.
  const userData = normalize(existingData) === normalize(path.join(appData, existingName))
    ? path.join(appData, LEGACY_DATA_DIRECTORY)
    : existingData
  app.setName(APP_NAME)
  fs.mkdirSync(userData, {recursive:true})
  app.setPath('userData', userData)
}

module.exports = { APP_NAME, LEGACY_DATA_DIRECTORY, configureAppIdentity }
