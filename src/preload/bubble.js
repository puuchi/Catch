const {contextBridge,ipcRenderer}=require('electron')
const listen=(channel,callback)=>{
  const receive=(_event,value)=>callback(value)
  ipcRenderer.on(channel,receive)
  return ()=>ipcRenderer.removeListener(channel,receive)
}
const action=(name,payload)=>ipcRenderer.invoke('catch:window',name,payload).catch(e=>console.error(e.message))
contextBridge.exposeInMainWorld('api',{
  getActivities:()=>ipcRenderer.invoke('catch:read','activity'),onActivities:fn=>listen('catch:activity',fn),
  getBubbleVisibility:()=>ipcRenderer.invoke('catch:read','visibility'),onBubbleVisibility:fn=>listen('catch:visibility',fn),
  moveWindow:(x,y)=>action('move',[x,y]),restoreMain:()=>action('expand'),showContextMenu:()=>action('menu')
})
