const path=require('node:path')
const fs=require('node:fs')
const {execFileSync}=require('node:child_process')
const root=path.resolve(__dirname,'..')
const frontend=path.resolve(root,'../frontend')
function build() {
  const esbuild=require('esbuild')
  fs.cpSync(path.join(root,'../frontend/public/assets/plants'),path.join(root,'assets/plants'),{recursive:true})
  esbuild.buildSync({entryPoints:[path.join(root,'src/preview-host.js')],outfile:path.join(root,'demo/model.mjs'),bundle:true,format:'esm',platform:'browser',target:'es2022',logLevel:'warning'})
  execFileSync(process.execPath,[path.join(frontend,'node_modules/vite/bin/vite.js'),'build','--config',path.join(frontend,'vite.demo.config.mjs')],{cwd:frontend,stdio:'inherit'})
  fs.cpSync(path.join(frontend,'public/assets'),path.join(root,'desktop-demo/assets'),{recursive:true})
  esbuild.buildSync({entryPoints:[path.join(root,'src/app.js')],outfile:path.join(root,'app.js'),bundle:true,platform:'browser',target:'es2022',legalComments:'inline',logLevel:'warning'})
  esbuild.buildSync({entryPoints:[path.join(root,'src/styles.css')],outfile:path.join(root,'app.css'),bundle:true,external:['assets/*'],logLevel:'warning'})
  for(const file of ['index.html','styles.css','assets/app-icon.png','assets/fonts/Manrope.ttf','assets/fonts/Fraunces.ttf','assets/sprout/growing-plant-sprite.png'])if(!fs.existsSync(path.join(root,file)))throw new Error('Missing site asset: '+file)
}
if(require.main===module){build();console.log('Catch site built locally.')}
module.exports={build}
