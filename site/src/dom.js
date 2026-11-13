const {icon}=require('./icons')
const {humanDuration}=require('./demo-state')

function element(tag,className,text) {
  const node=document.createElement(tag)
  if(className)node.className=className
  if(text!==undefined)node.textContent=text
  return node
}
function button(name,label,className='icon-button') {
  const node=element('button',className)
  node.type='button';node.setAttribute('aria-label',label);node.title=label;node.append(icon(name))
  return node
}
const compact=milliseconds=>humanDuration(milliseconds).replace(/\s/g,'')
const timeLabel=value=>new Date(value).toLocaleTimeString('zh-CN',{hour:'2-digit',minute:'2-digit',hour12:false})
module.exports={element,button,compact,timeLabel}
