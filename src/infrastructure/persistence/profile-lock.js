const fs=require('node:fs')
const path=require('node:path')

// Keep this handle open for the entire process lifetime. On Windows the Rust
// adapter opens the same file with share mode 0, which conflicts with both an
// existing Electron handle and any later attempt to open one. Electron's own
// single-instance lock excludes additional Electron writers.
class ProfileLock {
  constructor(directory) {
    fs.mkdirSync(directory,{recursive:true})
    try { this.descriptor=fs.openSync(path.join(directory,'.catch-write.lock'),'a+') }
    catch(cause) { throw new Error('数据目录正在被另一个版本使用，或无法锁定。请退出另一个 Catch 后重试。'+cause.message) }
  }
  close() {
    if(this.descriptor===undefined)return
    fs.closeSync(this.descriptor);this.descriptor=undefined
  }
}
module.exports={ProfileLock}
