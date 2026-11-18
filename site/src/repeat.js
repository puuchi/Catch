const repeatNames={daily:'每天',weekly:'每周',every:'间隔天数',monthly:'每月'}
const dayMs=86400000

function nextRepeatDate(rule,after) {
  const start=Date.parse(rule.start)
  const cutoff=Math.max(start-dayMs,after?Date.parse(after):0)
  if(rule.kind==='weekly' && rule.weekdays?.length) {
    let date=cutoff+dayMs
    while(!rule.weekdays.includes((new Date(date).getUTCDay()+6)%7))date+=dayMs
    return new Date(date).toISOString().slice(0,10)
  }
  if(rule.kind!=='monthly') {
    const step=(rule.kind==='weekly' ? 7 : rule.kind==='every' ? rule.interval : 1)*dayMs
    return new Date(start+(Math.floor((cutoff-start)/step)+1)*step).toISOString().slice(0,10)
  }
  const days=rule.monthDay ?? new Date(start).getUTCDate()
  const month=new Date(cutoff)
  month.setUTCDate(1)
  function occurrence() {
    const last=new Date(Date.UTC(month.getUTCFullYear(),month.getUTCMonth()+1,0)).getUTCDate()
    return +month+(Math.min(days,last)-1)*dayMs
  }
  if(occurrence()<=cutoff)month.setUTCMonth(month.getUTCMonth()+1)
  return new Date(occurrence()).toISOString().slice(0,10)
}

module.exports={nextRepeatDate,repeatNames}
