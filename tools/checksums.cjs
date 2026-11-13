const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto')
const {version}=require('../package.json')
const directory=path.resolve(__dirname,'../dist')

async function main() {
  const lines=[]
  for(const extension of ['exe','zip']) {
    const filename=`Catch-${version}-x64-win.${extension}`
    const hash=crypto.createHash('sha256')
    for await(const chunk of fs.createReadStream(path.join(directory,filename)))hash.update(chunk)
    lines.push(`${hash.digest('hex')}  ${filename}`)
  }
  fs.writeFileSync(path.join(directory,'SHA256SUMS.txt'),lines.join('\n')+'\n')
  console.log('Created dist/SHA256SUMS.txt for the Windows installer and ZIP.')
}

main().catch(error=>{console.error(error.message);process.exitCode=1})
