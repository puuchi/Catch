const path=require('node:path')
const {BrowserWindow,screen,Menu}=require('electron')
const {clampBounds}=require('./window-position')
const {showPaintedWindow}=require('./window-handoff')

class Windows {
  constructor(preferences,{quit,report}) {
    this.preferences=preferences;this.quit=quit;this.report=report
    this.views={};this.visible='main';this.wanted='main';this.transition=null;this.exiting=false
    this.positions=new Map();this.positionTimer=null
  }
  create(kind) {
    const small=kind==='bubble'
    const origin=this.views.main?.getBounds() || {x:100,y:100,width:320,height:480}
    const saved=this.preferences.get(small?'bubbleBounds':'windowBounds')
    const fallback=small?{x:origin.x+origin.width/2-18,y:origin.y,width:36,height:36}:origin
    const bounds=clampBounds(saved || fallback,screen.getAllDisplays())
    if(small)Object.assign(bounds,{width:36,height:36})
    const window=new BrowserWindow({
      ...bounds,show:false,frame:false,transparent:true,backgroundColor:'#00000000',alwaysOnTop:true,
      minWidth:small?36:260,minHeight:small?36:300,
      ...(small?{maxWidth:36,maxHeight:36,useContentSize:true,thickFrame:false,hasShadow:false}:{}),
      resizable:!small,maximizable:!small,minimizable:!small,fullscreenable:false,skipTaskbar:small,
      webPreferences:{nodeIntegration:false,contextIsolation:true,sandbox:true,backgroundThrottling:false,
        preload:path.resolve(__dirname,`../preload/${small?'bubble':'main'}.js`)}
    })
    this.views[kind]=window
    window.webContents.setWindowOpenHandler(()=>({action:'deny'}))
    window.webContents.on('will-navigate',event=>event.preventDefault())
    window.loadFile(path.resolve(__dirname,`../ui/${small?'bubble':'index'}.html`))
    window.on('moved',()=>this.schedulePosition(kind))
    window.on('resized',()=>this.schedulePosition(kind))
    window.on('close',event=>{if(!this.exiting){event.preventDefault();small?this.quit():this.show('bubble')}})
    window.on('closed',()=>{delete this.views[kind]})
    if(small) {
      let rejected=''
      const repair=()=>{
        if(window.isDestroyed())return
        const b=window.getBounds(),key=`${b.width}x${b.height}`
        if(Math.abs(b.width-36)<=2&&Math.abs(b.height-36)<=2){rejected='';return}
        if(key===rejected)return
        rejected=key;window.setBounds({...b,width:36,height:36})
      }
      window.on('resize',()=>setImmediate(repair))
      window.on('show',()=>{repair();window.webContents.send('catch:visibility',true)})
      window.on('hide',()=>window.webContents.send('catch:visibility',false))
    }
    return window
  }
  owner(contents) {return Object.keys(this.views).find(kind=>this.views[kind].webContents===contents)}
  send(topic,state) {for(const window of Object.values(this.views))if(!window.isDestroyed())window.webContents.send(topic,state)}
  schedulePosition(kind) {
    const window=this.views[kind];if(!window || window.isDestroyed())return
    const bounds=window.getBounds()
    this.positions.set(kind,kind==='bubble'?{...bounds,width:36,height:36}:bounds)
    clearTimeout(this.positionTimer)
    this.positionTimer=setTimeout(()=>{try{this.flush()}catch(e){this.report('窗口位置尚未保存',e)}},300)
  }
  flush() {
    clearTimeout(this.positionTimer)
    for(const [kind,bounds] of this.positions){this.preferences.set(kind==='bubble'?'bubbleBounds':'windowBounds',bounds);this.positions.delete(kind)}
  }
  fit() {
    for(const [kind,window] of Object.entries(this.views)) {
      const b=clampBounds(window.getBounds(),screen.getAllDisplays())
      if(kind==='main')window.setMinimumSize(Math.min(260,b.width),Math.min(300,b.height))
      window.setBounds(kind==='bubble'?{...b,width:36,height:36}:b);this.schedulePosition(kind)
    }
  }
  show(kind) {
    if(this.exiting)return Promise.resolve()
    this.wanted=kind
    if(this.transition)return this.transition
    this.transition=(async()=>{
      do {
        const target=this.wanted
        const incoming=this.views[target] || this.create(target)
        if(target==='main' && this.views.bubble?.isVisible()) {
          const anchor=this.views.bubble.getBounds()
          incoming.setBounds(clampBounds({...incoming.getBounds(),x:anchor.x,y:anchor.y},screen.getAllDisplays()))
        }
        await showPaintedWindow(incoming)
        if(this.exiting)return
        for(const [key,window] of Object.entries(this.views))if(key!==target)window.hide()
        this.visible=target
        if(target==='main')incoming.focus()
        this.schedulePosition(target)
      }while(this.wanted!==this.visible)
    })().catch(error=>{
      this.views[this.visible]?.show();this.wanted=this.visible
      for(const [key,window] of Object.entries(this.views))if(key!==this.visible)window.hide()
      this.report('窗口切换失败，已保留原窗口',error)
    }).finally(()=>{this.transition=null})
    return this.transition
  }
  action(owner,type,payload) {
    if(owner==='main') {
      if(type==='minimize')this.views.main.minimize()
      else if(type==='collapse')return this.show('bubble')
      else if(type==='pin' && typeof payload==='boolean')this.views.main.setAlwaysOnTop(payload)
      else if(type==='quit')this.quit()
      else throw new Error('未知窗口操作')
    } else if(owner==='bubble') {
      if(type==='expand')return this.show('main')
      if(type==='move' && Array.isArray(payload) && payload.length===2 && payload.every(Number.isFinite)) {
        this.views.bubble.setBounds(clampBounds({x:payload[0],y:payload[1],width:36,height:36},screen.getAllDisplays()))
      } else if(type==='menu') {
        Menu.buildFromTemplate([{label:'打开',click:()=>this.show('main')},{type:'separator'},{label:'关闭',click:()=>this.quit()}]).popup({window:this.views.bubble})
      } else throw new Error('无效的悬浮窗操作')
    } else throw new Error('未知窗口')
  }
}
module.exports={Windows}
