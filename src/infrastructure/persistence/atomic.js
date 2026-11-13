const fs = require('node:fs')
const path = require('node:path')
function atomicSave(file, data) {
  fs.mkdirSync(path.dirname(file), { recursive: true })
  const temporary = `${file}.tmp`
  let descriptor
  try {
    descriptor = fs.openSync(temporary, 'w')
    fs.writeFileSync(descriptor, JSON.stringify(data, null, 2), 'utf8')
    fs.fsyncSync(descriptor)
    fs.closeSync(descriptor)
    descriptor = undefined
    if (fs.existsSync(file)) fs.copyFileSync(file, `${file}.bak`)
    fs.renameSync(temporary, file)
  } finally {
    if (descriptor !== undefined) fs.closeSync(descriptor)
    if (fs.existsSync(temporary)) fs.unlinkSync(temporary)
  }
}


module.exports = { atomicSave }
