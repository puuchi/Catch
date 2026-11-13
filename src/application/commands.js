// @ts-check
class Commands {
  constructor() {this.pending=Promise.resolve();this.results=new Map()}
  /** @param {{requestId:string}} command @param {()=>any} execute */
  run(command,execute) {
    const fingerprint=JSON.stringify(command)
    const work=this.pending.then(()=>{
      const previous=this.results.get(command.requestId)
      if(previous) {
        if(previous.fingerprint!==fingerprint)throw Object.assign(new Error('重复请求参数不一致'),{code:'CONFLICT'})
        return structuredClone(previous.result)
      }
      const result=execute()
      this.results.set(command.requestId,{fingerprint,result:structuredClone(result)})
      if(this.results.size>512)this.results.delete(this.results.keys().next().value)
      return result
    })
    this.pending=work.then(()=>{},()=>{})
    return work
  }
  drain() {return this.pending}
}
module.exports={Commands}
