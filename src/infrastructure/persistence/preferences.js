const fs=require('node:fs')
const {atomicSave}=require('./atomic')
class Preferences {
  constructor(file) {
    this.file=file
    this.values=fs.existsSync(file)?JSON.parse(fs.readFileSync(file,'utf8')):{}
    if(!this.values || Array.isArray(this.values) || typeof this.values!=='object')throw new Error('配置文件格式无效，原文件已保留')
  }
  get(key) {return structuredClone(this.values[key])}
  set(key,value) {
    const next={...this.values,[key]:value}
    atomicSave(this.file,next)
    this.values=next
  }
}
module.exports={Preferences}
