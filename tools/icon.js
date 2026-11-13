// Reproducible code-native Catch icon, matching the application's check mark.
const fs=require('node:fs'),path=require('node:path'),zlib=require('node:zlib')
function crc32(bytes) {let value=0xffffffff;for(const byte of bytes){value^=byte;for(let i=0;i<8;i++)value=(value>>>1)^((value&1)?0xedb88320:0)}return (value^0xffffffff)>>>0}
function chunk(name,data) {const type=Buffer.from(name),size=Buffer.alloc(4),crc=Buffer.alloc(4);size.writeUInt32BE(data.length);crc.writeUInt32BE(crc32(Buffer.concat([type,data])));return Buffer.concat([size,type,data,crc])}
function distance(x,y,a,b) {const dx=b[0]-a[0],dy=b[1]-a[1],t=Math.max(0,Math.min(1,((x-a[0])*dx+(y-a[1])*dy)/(dx*dx+dy*dy)));return Math.hypot(x-a[0]-t*dx,y-a[1]-t*dy)}
function png(size) {
  const rows=Buffer.alloc(size*(size*4+1))
  for(let y=0;y<size;y++)for(let x=0;x<size;x++) {
    const px=(x+.5)*256/size,py=(y+.5)*256/size
    const rounded=Math.hypot(Math.max(62-px,0,px-194),Math.max(62-py,0,py-194))<=54
    const check=Math.min(distance(px,py,[65,130],[108,174]),distance(px,py,[108,174],[193,80]))<10
    const i=y*(size*4+1)+1+x*4
    rows.set(check?[249,248,241,255]:[46,90,73,rounded?255:0],i)
  }
  const header=Buffer.alloc(13);header.writeUInt32BE(size);header.writeUInt32BE(size,4);header[8]=8;header[9]=6
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',zlib.deflateSync(rows)),chunk('IEND',Buffer.alloc(0))])
}
const sizes=[16,32,48,64,128,256],images=sizes.map(png),head=Buffer.alloc(6+16*sizes.length)
head.writeUInt16LE(1,2);head.writeUInt16LE(sizes.length,4)
let offset=head.length
sizes.forEach((size,index)=>{const i=6+index*16;head[i]=head[i+1]=size===256?0:size;head.writeUInt16LE(1,i+4);head.writeUInt16LE(32,i+6);head.writeUInt32LE(images[index].length,i+8);head.writeUInt32LE(offset,i+12);offset+=images[index].length})
fs.writeFileSync(path.resolve(__dirname,'../assets/app-icon.ico'),Buffer.concat([head,...images]))
fs.writeFileSync(path.resolve(__dirname,'../assets/app-icon.png'),images.at(-1))
