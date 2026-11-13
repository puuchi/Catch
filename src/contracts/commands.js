// @ts-check
const todoTypes=new Set(['create','edit','complete','trash','restore','delete','archive'])
const activityTypes=new Set(['start','pause','resume','end','edit','remove','next-session','progress','matter-edit','session-edit','session-delete','session-add','trash-restore','trash-delete'])
/** @param {any} input @param {'todo'|'activity'} group */
function validateCommand(input,group) {
  if(group!=='todo'&&group!=='activity')throw new Error('未知命令分组')
  if(!input || typeof input!=='object' || typeof input.requestId!=='string' || !input.requestId || input.requestId.length>100)throw new Error('无效的请求标识')
  if(!(group==='todo'?todoTypes:activityTypes).has(input.type))throw new Error('未知命令')
  if(!Number.isInteger(input.expectedRevision) || input.expectedRevision<0)throw new Error('缺少数据版本')
  if(!input.payload || typeof input.payload!=='object' || Array.isArray(input.payload))throw new Error('命令参数无效')
  if(JSON.stringify(input).length>100000)throw new Error('操作内容过大')
  return input
}
module.exports={validateCommand}
