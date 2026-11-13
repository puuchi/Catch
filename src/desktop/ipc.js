const {ipcMain}=require('electron')
const {validateCommand}=require('../contracts/commands')
const {executeTracking}=require('../application/tracking/commands')
function installIPC({windows,todos,tracking,preferences,commands,isClosing}) {
  ipcMain.handle('catch:read',(event,name)=>{
    const owner=windows.owner(event.sender)
    if(!owner)throw new Error('未知窗口')
    if(name==='activity')return tracking.snapshot()
    if(name==='visibility' && owner==='bubble')return windows.views.bubble.isVisible()
    if(owner!=='main')throw new Error('窗口无此权限')
    if(name==='todos')return todos.snapshot()
    if(name==='appearance')return preferences.get('backgroundTransparency') ?? 45
    throw new Error('未知查询')
  })
  ipcMain.handle('catch:command',async(event,group,raw)=>{
    if(windows.owner(event.sender)!=='main')return {ok:false,error:'窗口无此权限',code:'FORBIDDEN'}
    if(isClosing())return {ok:false,error:'正在退出，请稍后操作',code:'CONFLICT'}
    try {
      const command=validateCommand(raw,group)
      const result=await commands.run({...command,group},()=>{
        const state=group==='todo'?todos.command(command.type,command.payload,command.expectedRevision):executeTracking(tracking,todos,command)
        windows.send(group==='todo'?'catch:todos':'catch:activity',state)
        return {ok:true,state}
      })
      return result
    } catch(error) {
      const state=group==='todo'?todos.snapshot():tracking.snapshot()
      return {ok:false,error:error.message,code:error.code || 'VALIDATION',state}
    }
  })
  ipcMain.handle('catch:appearance',async(event,value)=>{
    if(windows.owner(event.sender)!=='main')return {ok:false,error:'窗口无此权限'}
    try {
      if(!Number.isInteger(value)||value<0||value>100)throw new Error('透明度须介于 0% 和 100% 之间')
      await commands.run({requestId:require('node:crypto').randomUUID()},()=>preferences.set('backgroundTransparency',value))
      return {ok:true}
    }catch(e){return {ok:false,error:e.message}}
  })
  ipcMain.handle('catch:window',(event,type,payload)=>windows.action(windows.owner(event.sender),type,payload))
}
module.exports={installIPC}
