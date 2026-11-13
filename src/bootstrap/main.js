const {app,dialog,powerMonitor,screen}=require('electron')
const path=require('node:path')
const {randomUUID}=require('node:crypto')
const {configureAppIdentity}=require('../desktop/app-identity')
const {TrackingRepository}=require('../infrastructure/persistence/tracking-repository')
const {TodosRepository}=require('../infrastructure/persistence/todos-repository')
const {Preferences}=require('../infrastructure/persistence/preferences')
const {TrackingService}=require('../application/tracking/service')
const {TodosService}=require('../application/todos/service')
const {Commands}=require('../application/commands')
const {Windows}=require('../desktop/windows')
const {installIPC}=require('../desktop/ipc')

if(process.env.CATCH_TEST_DATA)app.setPath('userData',path.resolve(process.env.CATCH_TEST_DATA))
configureAppIdentity(app)
let exiting=false,closing=false,timer,services
function report(context,error) {
  console.error(context,error.message)
  if(!process.env.CATCH_TEST_DATA)dialog.showErrorBox('Catch',`${context}\n${error.message}`)
}
async function quit() {
  if(closing||exiting||!services)return
  closing=true
  const {windows,tracking,commands}=services
  try {
    await commands.drain()
    if(tracking.current()) {
      const result=await dialog.showMessageBox(windows.views[windows.visible],{type:'question',title:'退出 Catch',message:'正在计时，是否暂停并退出？',buttons:['取消','暂停并退出'],defaultId:0,cancelId:0,noLink:true})
      if(result.response!==1)return
    }
    tracking.pause('quit');tracking.checkpoint();windows.flush()
    clearInterval(timer);windows.exiting=true;exiting=true;app.quit()
  }catch(error){report('退出前保存失败，应用尚未退出',error)}
  finally{closing=false}
}
const ready=app.requestSingleInstanceLock()?app.whenReady().then(async()=>{
  const directory=app.getPath('userData'),dependencies={now:Date.now,newId:randomUUID}
  const tracking=new TrackingService(new TrackingRepository(path.join(directory,'activity-log.json')),dependencies)
  const todos=new TodosService(new TodosRepository(directory),dependencies)
  const preferences=new Preferences(path.join(directory,'config.json'))
  const commands=new Commands()
  const windows=new Windows(preferences,{quit,report})
  services={tracking,todos,preferences,commands,windows}
  installIPC({...services,isClosing:()=>closing||exiting})
  const interrupt=reason=>commands.run({requestId:randomUUID()},()=>windows.send('catch:activity',tracking.interrupt(reason)))
  powerMonitor.on('lock-screen',()=>void interrupt('lock'))
  powerMonitor.on('suspend',()=>void interrupt('sleep'))
  powerMonitor.on('shutdown',()=>void interrupt('shutdown'))
  screen.on('display-added',()=>windows.fit())
  screen.on('display-removed',()=>windows.fit())
  screen.on('display-metrics-changed',()=>windows.fit())
  windows.create('main').on('session-end',()=>void interrupt('shutdown'))
  timer=setInterval(()=>{
    if(closing)return
    void commands.run({requestId:randomUUID()},()=>{
      try{tracking.checkpoint()}catch(error){tracking.interrupt('save-error')}
      windows.send('catch:activity',tracking.snapshot())
    }).catch(error=>report('检查点保存失败',error))
  },30000)
  await windows.show('main')
  return services
}).catch(error=>{report('启动失败，原数据已保留',error);app.exit(1);throw error}):(app.quit(),Promise.resolve(null))
app.on('second-instance',()=>services?.windows.show('main'))
app.on('activate',()=>services?.windows.show('main'))
app.on('before-quit',event=>{if(!exiting && services){event.preventDefault();void quit()}})
app.on('window-all-closed',()=>{if(exiting)app.quit()})
module.exports={ready}
