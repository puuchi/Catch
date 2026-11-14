import type {Matter,Segment,Session,Todo} from './models';
export const day=(time=Date.now())=>{const d=new Date(time);return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`};
export const bounds=(date:string):[number,number]=>{const start=new Date(`${date}T00:00:00`);const end=new Date(start);end.setDate(end.getDate()+1);return [+start,+end]};
export const effective=(segments:Segment[],now:number,range:[number,number]=[-Infinity,Infinity])=>segments.reduce((n,s)=>n+Math.max(0,Math.min(s.end===null?now:Date.parse(s.end),range[1])-Math.max(Date.parse(s.start),range[0])),0);
export const duration=(item:{segments:Segment[]},now:number,range?:[number,number])=>effective(item.segments,now,range);
export const clock=(ms:number)=>{const s=Math.max(0,Math.floor(ms/1000));return [Math.floor(s/3600),Math.floor(s/60)%60,s%60].map(n=>String(n).padStart(2,'0')).join(':')};
export const human=(ms:number)=>{const s=Math.max(0,Math.floor(ms/1000)),h=Math.floor(s/3600),m=Math.floor(s/60)%60;return [h?`${h} 小时`:'',m?`${m} 分`:'',s%60||!s?`${s%60} 秒`:''].filter(Boolean).join(' ')};
export const time=(iso:string|null)=>iso?new Date(iso).toLocaleTimeString('zh-CN',{hour12:false,hour:'2-digit',minute:'2-digit'}):'现在';
export const inputTime=(iso:string)=>{const d=new Date(iso);return `${day(+d)}T${String(d.getHours()).padStart(2,'0')}:${String(d.getMinutes()).padStart(2,'0')}:${String(d.getSeconds()).padStart(2,'0')}.${String(d.getMilliseconds()).padStart(3,'0')}`};
export const statusText=(status?:string)=>({running:'正在投入',paused:'准备继续',completed:'这件事告一段落'}[status||'']||'从一件小事开始');
export const statusShort=(status: string)=>({running:'进行中',paused:'已暂停',completed:'已完成'}[status]||'');
export const foreground=(items:Matter[],retained:string|null)=>items.find(i=>i.status==='running')||items.find(i=>i.id===retained)||items.filter(i=>i.status==='paused').sort((a,b)=>Date.parse(b.updatedAt)-Date.parse(a.updatedAt))[0]||null;
export const visibleTodos=(rows:Todo[],today:string)=>rows.filter(t=>!t.deletedAt&&(!t.done||t.completedAt===today)).sort((a,b)=>rank(a,today)-rank(b,today));
function rank(t:Todo,today:string){return t.done?3:t.urgent?0:t.deadline===today?1:2}
export interface Entry{matter:Matter;session:Session;start:number;end:number;duration:number}
export function entries(items:Matter[],date:string,now:number):Entry[]{
  const range=bounds(date);
  return items.flatMap(matter=>matter.sessions.map(session=>({matter,session,start:Math.max(Date.parse(session.startTime),range[0]),end:Math.min(session.endTime?Date.parse(session.endTime):now,range[1]),duration:effective(session.segments,now,range)})))
    .filter(e=>e.duration>0||(Date.parse(e.session.startTime)>=range[0]&&Date.parse(e.session.startTime)<range[1]&&(!e.session.segments.length||e.session.segments.every(s=>s.start===s.end)||e.session.endTime===null)))
    .sort((a,b)=>(date===day(now)?-1:1)*(a.start-b.start));
}
export function gaps(items:Matter[],date:string,now:number){
  const range=bounds(date),merged:{start:number;end:number}[]=[];
  const intervals=items.flatMap(m=>m.segments.map(s=>({start:Math.max(Date.parse(s.start),range[0]),end:Math.min(s.end?Date.parse(s.end):now,range[1])}))).filter(s=>s.end>s.start).sort((a,b)=>a.start-b.start);
  for(const i of intervals){const last=merged.at(-1);if(last&&i.start<=last.end)last.end=Math.max(last.end,i.end);else merged.push({...i})}
  const result=merged.slice(1).map((s,i)=>({start:merged[i].end,end:s.start})).filter(s=>s.end-s.start>=1000);
  const last=merged.at(-1);if(date===day(now)&&last&&now-last.end>=1000)result.push({start:last.end,end:now});return result;
}
export function resizedSegments(session:Session,start:string,end:string):Segment[]{
  const from=Date.parse(start),to=Date.parse(end),oldStart=Date.parse(session.startTime),oldEnd=Date.parse(session.endTime!);
  if(from===oldStart&&to===oldEnd)return session.segments.map(s=>({...s}));
  const shift=from-oldStart;
  if(shift===to-oldEnd)return session.segments.map(s=>({start:new Date(Date.parse(s.start)+shift).toISOString(),end:new Date(Date.parse(s.end!)+shift).toISOString()}));
  return session.segments.map((s,i)=>({start:Math.max(from,i===0&&Date.parse(s.start)===oldStart?from:Date.parse(s.start)),end:Math.min(to,i===session.segments.length-1&&Date.parse(s.end!)===oldEnd?to:Date.parse(s.end!))})).filter(s=>s.end>s.start).map(s=>({start:new Date(s.start).toISOString(),end:new Date(s.end).toISOString()}));
}
