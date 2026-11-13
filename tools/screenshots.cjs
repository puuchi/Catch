// Regenerates README images from synthetic records in a temporary profile.
const fs=require('node:fs'),path=require('node:path'),os=require('node:os')
if(!process.versions.electron) {
  require('./build')
  const temporaryRoot=fs.realpathSync(os.tmpdir())
  const directory=fs.mkdtempSync(path.join(temporaryRoot,'catch-showcase-'))
  const env={...process.env,CATCH_TEST_DATA:directory};delete env.ELECTRON_RUN_AS_NODE
  try {
    const result=require('node:child_process').spawnSync(require('electron'),[__filename],{env,windowsHide:true,stdio:'inherit',timeout:30000})
    if(result.status!==0)throw new Error('Screenshot capture failed')
  }finally{
    const resolved=path.resolve(directory)
    if(path.dirname(resolved)!==temporaryRoot || !path.basename(resolved).startsWith('catch-showcase-'))throw new Error('Unexpected screenshot profile path')
    fs.rmSync(resolved,{recursive:true,force:true,maxRetries:5,retryDelay:200})
  }
} else {
  const {app}=require('electron')
  const {TrackingModel}=require('../src/domain/tracking/model')
  app.commandLine.appendSwitch('force-device-scale-factor','2')
  const directory=process.env.CATCH_TEST_DATA
  if(!directory)throw new Error('A temporary screenshot profile is required')
  fs.writeFileSync(path.join(directory,'todos.json'),JSON.stringify([
    {id:'design',text:'完善 Catch 的交互细节',notes:'保留简单、轻盈的使用体验',urgent:true,done:false,deadline:null},
    {id:'docs',text:'整理项目说明和发布文档',notes:'让第一次使用的人也能轻松上手',urgent:false,done:false,deadline:null},
    {id:'walk',text:'出去散步，换个思路',notes:'',urgent:false,done:false,deadline:null}
  ]))
  fs.writeFileSync(path.join(directory,'config.json'),JSON.stringify({windowBounds:{x:100,y:100,width:320,height:560},backgroundTransparency:0}))
  const yesterday=new Date();yesterday.setDate(yesterday.getDate()-1)
  let tick=0,sequence=0
  const at=(hour,minute,second=0)=>{const date=new Date(yesterday);date.setHours(hour,minute,second,0);tick=+date}
  const fixture=new TrackingModel({version:2,sessionVersion:1,checkpointAt:null,activities:[],trash:[]},{now:()=>tick,newId:()=>`doc-matter-${++sequence}`})
  at(9,0);fixture.start('完善 Catch 的交互细节','design')
  const mainId=fixture.current().id
  at(9,20);fixture.setProgress(mainId,fixture.current().sessions.at(-1).id,'整理了待办和计时的交互');fixture.pause()
  at(9,40);fixture.resume(mainId)
  at(9,57,20);fixture.setProgress(mainId,fixture.current().sessions.at(-1).id,'完成前台保留与重新开始');fixture.pause()
  at(10,30);fixture.start('整理项目说明和发布文档','docs')
  at(10,38);fixture.setProgress(fixture.current().id,fixture.current().sessions.at(-1).id,'梳理安装与使用说明');fixture.pause()
  at(11,10);fixture.start('阅读与思考')
  at(11,16);fixture.setProgress(fixture.current().id,fixture.current().sessions.at(-1).id,'留下一两个自己的想法');fixture.pause()
  tick=Date.now();fixture.commit(()=>{})
  fs.writeFileSync(path.join(directory,'activity-log.json'),JSON.stringify(fixture.snapshot()))

  const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms))
  const until=async condition=>{for(let i=0;i<120;i++){if(await condition())return;await delay(25)}throw new Error('Screenshot view did not become ready')}
  require('../src/bootstrap/main').ready.then(async({tracking,windows})=>{
    const main=windows.views.main
    const evaluate=source=>main.webContents.executeJavaScript(source,true)
    await until(()=>evaluate("Boolean(document.getElementById('currentStatusText') && window.api)"))
    await evaluate('document.fonts.ready.then(()=>true)')
    const out=path.resolve(__dirname,'../docs/screenshots');fs.mkdirSync(out,{recursive:true})
    const capture=async page=>{
      await evaluate(`document.querySelector('[data-page=${page}]').click()`)
      await delay(100)
      fs.writeFileSync(path.join(out,page+'.png'),(await main.webContents.capturePage()).toPNG())
    }
    await capture('todos')
    // Make the retained card display the completed matter before capturing it.
    tracking.resume(mainId);windows.send('catch:activity',tracking.snapshot())
    await until(()=>evaluate("document.getElementById('currentStatusText').textContent==='正在投入'"))
    tracking.setProgress(mainId,tracking.current().sessions.at(-1).id,'已完成主要交互，下一次继续打磨文档')
    tracking.end(mainId);windows.send('catch:activity',tracking.snapshot())
    await until(()=>evaluate("!document.getElementById('completedSummary').hidden"))
    await capture('current')
    await evaluate("document.querySelector('[data-page=timeline]').click()")
    const day=require('../src/domain/tracking/time').localDate(yesterday)
    await evaluate(`document.getElementById('timelineDate').value=${JSON.stringify(day)};document.getElementById('timelineDate').dispatchEvent(new Event('change'))`)
    await capture('timeline')
    console.log('Updated README screenshots: todos, current (completed summary), timeline; synthetic data only.')
    app.quit()
  }).catch(error=>{console.error(error.stack);app.exit(1)})
}
